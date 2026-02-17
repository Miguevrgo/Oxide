use crate::board::Board;
use crate::moves::{Move, MoveList};
use crate::piece::Colour;
use crate::search::{
    HISTORY_FACTOR, HISTORY_MAX_BONUS, HISTORY_OFFSET, INF, LMR_BASE, LMR_DIV, MATE, MAX_DEPTH,
    MAX_HISTORY,
};
use std::time::Instant;

use super::network::EvalTable;
use super::search::MAX_CAP_HISTORY;

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

/// History Gravity bonus
/// https://www.chessprogramming.org/History_Heuristic
pub fn history_bonus(depth: u8) -> i16 {
    HISTORY_MAX_BONUS.min(HISTORY_FACTOR * depth as i16 - HISTORY_OFFSET)
}

/// Taper history so it clamps to MAX
/// From Carp, which in turn is from talkchess
const fn taper_bonus<const MAX: i32>(bonus: i16, old: i16) -> i16 {
    let o = old as i32;
    let b = bonus as i32;

    (o + b - (o * b.abs()) / MAX) as i16
}

pub struct HistoryTable {
    pub score: [[[i16; 64]; 64]; 2], // [side][src][dest]
}

impl HistoryTable {
    /// Updating history values, for the cutoff move a bonus and for the rest of the quiets tried,
    /// a history maluse, using history gravity formula
    pub fn update(&mut self, side: Colour, src: usize, dest: usize, bonus: i16, quiets: &[Move]) {
        let c_bonus = bonus.clamp(-MAX_HISTORY as i16, MAX_HISTORY as i16);

        // Update the current best move with positive bonus
        let old_score = &mut self.score[side as usize][src][dest];
        *old_score = taper_bonus::<MAX_HISTORY>(c_bonus, *old_score);

        // Update all other quiet moves with negative bonus
        for m in quiets {
            let old = &mut self.score[side as usize][m.get_source().index()][m.get_dest().index()];
            *old = taper_bonus::<MAX_HISTORY>(-c_bonus, *old);
        }
    }
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self {
            score: [[[0; 64]; 64]; 2],
        }
    }
}

pub struct CaptureHistoryTable {
    pub score: [[[i16; 5]; 64]; 12], // [capturing_piece][dest][captured]
}

impl CaptureHistoryTable {
    pub fn update(&mut self, board: &Board, m: Move, bonus: i16, captures: &[Move]) {
        let c_bonus = bonus.clamp(-MAX_CAP_HISTORY as i16, MAX_CAP_HISTORY as i16);

        // Update the best move with a positive bonus
        if m.get_type().is_capture() {
            let old_score = &mut self.score[board.piece_at(m.get_source()) as usize]
                [m.get_dest().index()][board.capture_piece(m).index()];
            *old_score = taper_bonus::<MAX_CAP_HISTORY>(c_bonus, *old_score);
        }

        // Update all other capture moves with negative bonus
        for mov in captures {
            let old = &mut self.score[board.piece_at(mov.get_source()) as usize]
                [mov.get_dest().index()][board.capture_piece(*mov).index()];
            *old = taper_bonus::<MAX_CAP_HISTORY>(-c_bonus, *old);
        }
    }
}

impl Default for CaptureHistoryTable {
    fn default() -> Self {
        Self {
            score: [[[0; 5]; 64]; 12],
        }
    }
}

pub struct LmrTable {
    pub base: [[i16; MoveList::SIZE + 1]; MAX_DEPTH as usize + 1],
}

impl LmrTable {
    pub fn new() -> Self {
        let log_depth: Vec<f64> = (0..=MAX_DEPTH)
            .map(|d| if d > 0 { (d as f64).ln() } else { 0.0 })
            .collect();

        let log_move: Vec<f64> = (0..=MoveList::SIZE)
            .map(|m| if m > 0 { (m as f64).ln() } else { 0.0 })
            .collect();

        let mut table = [[0i16; MoveList::SIZE + 1]; (MAX_DEPTH + 1) as usize];

        for (d, &ld) in log_depth.iter().enumerate() {
            for (m, &lm) in log_move.iter().enumerate() {
                table[d][m] = (LMR_BASE + ld / LMR_DIV * lm) as i16;
            }
        }

        table[0][0] = 0;
        table[1][0] = 0;
        table[0][1] = 0;

        Self { base: table }
    }
}

pub const MAX_PLY: usize = 128;

#[derive(Clone, Copy, Default)]
pub struct PlyData {
    pub killer: Move,
    pub eval: i32,
    pub pv: MoveList,
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
    pub history: HistoryTable,
    pub cap_history: CaptureHistoryTable,
    pub lmr_table: LmrTable,
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

            stack: Vec::with_capacity(32),
            ply_data: [(); MAX_PLY].map(|_| PlyData::default()),
            tt: TranspositionTable::with_size_mb(32),
            cache: EvalTable::default(),
            history: HistoryTable::default(),
            cap_history: CaptureHistoryTable::default(),
            lmr_table: LmrTable::new(),
        }
    }

    pub fn start_search(&mut self) {
        self.depth = 1;
        self.stop = false;
        self.best_move = Move::NULL;
        self.nodes = 0;
        self.ply = 0;
        self.timing = Instant::now();
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

    pub fn is_repetition(&self, board: &Board, curr_hash: u64, root: bool) -> bool {
        if self.stack.len() < 6 {
            return false;
        }

        let mut reps = 1 + u8::from(root);
        for &hash in self
            .stack
            .iter()
            .rev()
            .take(usize::from(board.halfmoves + 1))
            .skip(1)
            .step_by(2)
        {
            reps -= u8::from(hash == curr_hash);
            if reps == 0 {
                return true;
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
                "info depth {} score mate {sign}{mate_in} time {time} nodes {} nps {nps} pv{}",
                self.depth, self.nodes, self.ply_data[0].pv
            )
        } else {
            write!(
                f,
                "info depth {} score cp {} time {time} nodes {} nps {nps} pv{}",
                self.depth, self.eval, self.nodes, self.ply_data[0].pv
            )
        }
    }
}
