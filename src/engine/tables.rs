use crate::engine::search::{INF, MATE, MAX_DEPTH};
use crate::game::moves::Move;
use std::time::Instant;

use super::network::EvalTable;

/// Transposition Table
#[derive(Copy, Clone, PartialEq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct TTEntry {
    pub key: u64,
    pub value: i32,
    pub best_move: Move,
    pub age: u8,
    pub flags: u8, // depth(6) + bound(2)
}

impl TTEntry {
    #[inline]
    pub fn depth(&self) -> u8 {
        self.flags >> 2
    }

    #[inline]
    pub fn bound(&self) -> Bound {
        match self.flags & 0b11 {
            1 => Bound::Lower,
            2 => Bound::Upper,
            _ => Bound::Exact,
        }
    }

    #[inline]
    pub fn make_flags(depth: u8, bound: Bound) -> u8 {
        ((depth.min(63)) << 2) | (bound as u8 & 0b11)
    }
}

pub struct TranspositionTable {
    pub tt: Vec<TTEntry>,
    age: u8,
}

impl TranspositionTable {
    pub fn with_size_mb(mb: usize) -> Self {
        let bytes = mb * 1_048_576;
        let entry_sz = std::mem::size_of::<TTEntry>();
        let len = (bytes / entry_sz).next_power_of_two();
        Self {
            tt: vec![TTEntry::default(); len],
            age: 0,
        }
    }

    fn idx(&self, hash: u64) -> usize {
        // (Read Lemire Blog for explanation | Carp)
        ((hash as u128 * self.tt.len() as u128) >> 64) as usize
    }

    pub fn probe(&self, hash: u64) -> Option<&TTEntry> {
        let e = &self.tt[self.idx(hash)];
        (e.key == hash).then_some(e)
    }

    pub fn clear(&mut self) {
        self.tt.fill(TTEntry::default());
        self.age = 0;
    }

    pub fn inc_age(&mut self) {
        self.age = (self.age + 1) & 0x7F;
    }

    pub fn insert(
        &mut self,
        hash: u64,
        bound: Bound,
        mut best: Move,
        value: i32,
        depth: u8,
        pv: bool,
    ) {
        let idx = self.idx(hash);
        let slot = &mut self.tt[idx];
        let same = slot.key == hash;

        if self.age != slot.age
            || !same
            || bound == Bound::Exact
            || depth as usize + 4 + 2 * pv as usize > slot.depth() as usize
        {
            if best == Move::NULL && same {
                best = slot.best_move;
            }

            *slot = TTEntry {
                key: hash,
                value,
                best_move: best,
                age: self.age,
                flags: TTEntry::make_flags(depth, bound),
            };
        }
    }
}

pub const MAX_PLY: usize = 128;

#[derive(Clone, Copy, Default)]
pub struct PlyData {
    pub killers: [Move; 2],
    pub eval: i32,
}

pub struct SearchData {
    // Search Control
    pub timing: Instant,
    pub time_tp: u128,
    pub stop: bool,
    pub depth: u8,

    // Data
    pub ply: usize,
    pub nodes: u64,
    pub best_move: Move,
    pub eval: i32,

    // Tables + Ordering
    pub stack: Vec<u64>,
    pub ply_data: [PlyData; MAX_PLY],
    pub tt: TranspositionTable,
    pub cache: EvalTable,
    pub history: [[[i16; 64]; 64]; 2], // [colour][src][dest]
}

impl SearchData {
    pub fn new() -> Self {
        Self {
            timing: Instant::now(),
            time_tp: 0,
            stop: false,
            depth: 0,

            ply: 0,
            nodes: 0,
            best_move: Move::NULL,
            eval: -INF,

            stack: Vec::with_capacity(16),
            ply_data: [(); MAX_PLY].map(|_| PlyData::default()),
            tt: TranspositionTable::with_size_mb(16),
            cache: EvalTable::default(),
            history: [[[0; 64]; 64]; 2],
        }
    }

    pub fn start_search(&mut self) {
        self.depth = 1;
        self.stop = false;
        self.best_move = Move::NULL;
        self.nodes = 0;
        self.ply = 0;
        self.timing = Instant::now();
        self.decay_history();
    }

    fn decay_history(&mut self) {
        self.history
            .iter_mut()
            .flatten()
            .flatten()
            .for_each(|v| *v /= 2);
    }

    pub fn push(&mut self, hash: u64) {
        self.ply += 1;
        self.stack.push(hash);
    }

    pub fn pop(&mut self) {
        self.stack.pop();
        self.ply -= 1;
    }

    pub fn resize_tt(&mut self, mb_size: usize) {
        self.tt = TranspositionTable::with_size_mb(mb_size);
    }

    pub fn clear(&mut self) {
        self.stack.clear();
        self.nodes = 0;
        self.ply = 0;
    }

    pub fn is_repetition(&self, curr_hash: u64, root: bool) -> bool {
        if self.stack.len() < 6 {
            return false;
        }

        let mut reps = 1 + u8::from(root);
        for &hash in self.stack.iter().rev().skip(1).step_by(2) {
            if hash == curr_hash {
                reps -= 1;
                if reps == 0 {
                    return true;
                }
            }
        }
        false
    }

    pub fn continue_search(&self) -> bool {
        let time = self.timing.elapsed().as_millis();
        time < self.time_tp
    }
}

impl std::fmt::Display for SearchData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let time = self.timing.elapsed().as_millis();
        let nps = if time > 0 {
            (1000 * self.nodes as u128 / time) as u64
        } else {
            0
        };

        if self.eval.abs() >= MATE - i32::from(MAX_DEPTH) {
            let mate_in = (MATE - self.eval.abs()) / 2;
            let sign = if self.eval < 0 { "-" } else { "" };
            write!(
                f,
                "info depth {} score mate {sign}{mate_in} time {time} nodes {} nps {nps} pv {}",
                self.depth, self.nodes, self.best_move
            )
        } else {
            write!(
                f,
                "info depth {} score cp {} time {time} nodes {} nps {nps} pv {}",
                self.depth, self.eval, self.nodes, self.best_move
            )
        }
    }
}
