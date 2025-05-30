use std::collections::HashMap;

#[derive(Copy, Clone)]
pub enum Bound {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Copy, Clone)]
pub struct TTEntry {
    pub depth: usize,
    pub value: i32,
    pub bound: Bound,
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
