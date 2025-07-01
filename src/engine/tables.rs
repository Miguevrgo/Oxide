use crate::game::moves::Move;
use std::collections::HashMap;

/// Transposition Table
#[derive(Copy, Clone)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Copy, Clone)]
pub struct TTEntry {
    pub depth: usize,
    pub value: i32,
    pub bound: Bound,
    pub best_move: Move,
}

pub struct TranspositionTable {
    tt: HashMap<u64, TTEntry>,
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

pub const MAX_PLY: usize = 64;

#[derive(Clone, Copy, Default)]
pub struct PlyData {
    pub killers: [Move; 2],
}

pub struct SearchTables {
    pub ply: [PlyData; MAX_PLY],
}

impl SearchTables {
    pub fn new() -> Self {
        Self {
            ply: [(); MAX_PLY].map(|_| PlyData::default()),
        }
    }
}
