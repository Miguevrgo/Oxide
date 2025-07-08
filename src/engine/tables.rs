use crate::engine::search::INF;
use crate::game::moves::Move;
use std::{collections::HashMap, time::Instant};

use super::network::EvalTable;

/// Transposition Table
#[derive(Copy, Clone)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TTEntry {
    pub value: i32,
    pub best_move: Move,
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
    pub tt: HashMap<u64, TTEntry>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        TranspositionTable {
            tt: HashMap::default(),
        }
    }

    pub fn get(&self, key: u64) -> Option<&TTEntry> {
        self.tt.get(&key)
    }

    pub fn insert(&mut self, key: u64, entry: TTEntry) {
        self.tt.insert(key, entry);
    }
}

pub const MAX_PLY: usize = 512;

#[derive(Clone, Copy, Default)]
pub struct PlyData {
    pub killers: [Move; 2],
    pub eval: i32,
}

pub struct SearchData {
    pub timing: Instant,
    pub ply: usize,
    pub nodes: u64,
    pub best_move: Move,
    pub eval: i32,

    // Tables + Ordering
    pub stack: Vec<u64>,
    pub ply_data: [PlyData; MAX_PLY],
    pub tt: TranspositionTable,
    pub cache: EvalTable,
}

impl SearchData {
    pub fn new() -> Self {
        Self {
            timing: Instant::now(),
            ply: 0,
            nodes: 0,
            best_move: Move::NULL,
            eval: -INF,
            stack: Vec::with_capacity(16),
            ply_data: [(); MAX_PLY].map(|_| PlyData::default()),
            tt: TranspositionTable::new(),
            cache: EvalTable::default(),
        }
    }

    pub fn push(&mut self, hash: u64) {
        self.ply += 1;
        self.stack.push(hash);
    }

    pub fn pop(&mut self) {
        self.stack.pop();
        self.ply -= 1;
    }

    pub fn clear(&mut self) {
        self.stack.clear();
        self.tt.tt.clear(); // TODO
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
}
