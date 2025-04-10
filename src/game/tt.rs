use std::sync::atomic::{AtomicU64, Ordering};

use crate::game::moves::Move;
use crate::game::zobrist::ZHash;

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash, Default)]
pub enum TTFlag {
    #[default]
    UpperBound,
    LowerBound,
    Exact,
}

/// `TTEntry`: uncompressed external representation of later compressed 128b
///
///
/// 00000000 00000000 00000000 00000000 00000000 00000000 00000000 01111111 AGE
/// 00000000 00000000 00000000 00000000 00000000 00000000 00111111 10000000 DEPTH
/// 00000000 00000000 00000000 00000000 00000000 00000000 11000000 00000000 FLAG
/// 00000000 00000000 00000000 00000000 11111111 11111111 00000000 00000000 MOVE
/// 00000000 00000000 11111111 11111111 00000000 00000000 00000000 00000000 STATIC EVAL
/// 11111111 11111111 00000000 00000000 00000000 00000000 00000000 00000000 SEARCH VALUE
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash, Default)]
pub struct TTEntry {
    key: u64,
    age: u8,
    depth: u8,
    flag: TTFlag,
    best_move: Move,
    eval: i16,
    value: i16,
}

const DEPTH_OFFSET: u64 = 7;
const FLAG_OFFSET: u64 = 14;
const MOVE_OFFSET: u64 = 16;
const EVAL_OFFSET: u64 = 32;
const VALUE_OFFSET: u64 = 48;

const AGE_MASK: u64 = 0x7F;
const DEPTH_MASK: u64 = 0x3F80;
const FLAG_MASK: u64 = 0xC000;
const MOVE_MASK: u64 = 0xFFFF0000;
const EVAL_MASK: u64 = 0xFFFF00000000;

const LONGEST_TB_MATE: i32 = 16384;

impl TTEntry {
    pub fn get_depth(self) -> usize {
        self.depth as usize
    }

    pub fn get_flag(self) -> TTFlag {
        self.flag
    }

    pub fn get_move(self) -> Option<Move> {
        if self.best_move != Move::default() {
            Some(self.best_move)
        } else {
            None
        }
    }

    pub fn get_eval(self) -> i32 {
        self.eval as i32
    }

    pub fn get_value(self, ply: usize) -> i32 {
        let value = self.value as i32;
        if value >= LONGEST_TB_MATE {
            value - ply as i32
        } else if value <= -LONGEST_TB_MATE {
            value + ply as i32
        } else {
            value
        }
    }
}

impl From<TTEntry> for (u64, u64) {
    fn from(entry: TTEntry) -> Self {
        let data: u64 = entry.age as u64
            | ((entry.depth as u64) << DEPTH_OFFSET)
            | ((entry.flag as u64) << FLAG_OFFSET)
            | ((entry.best_move.0 as u64) << MOVE_OFFSET)
            | ((entry.eval as u16 as u64) << EVAL_OFFSET)
            | ((entry.value as u16 as u64) << VALUE_OFFSET);
        (entry.key ^ data, data)
    }
}

impl From<(u64, u64)> for TTEntry {
    fn from((key, data): (u64, u64)) -> Self {
        Self {
            key: key ^ data,
            age: (data & AGE_MASK) as u8,
            depth: ((data & DEPTH_MASK) >> DEPTH_OFFSET) as u8,
            flag: unsafe { std::mem::transmute(((data & FLAG_MASK) >> FLAG_OFFSET) as u8) },
            best_move: Move(((data & MOVE_MASK) >> MOVE_OFFSET) as u16),
            eval: ((data & EVAL_MASK) >> EVAL_OFFSET) as i16,
            value: (data >> VALUE_OFFSET) as i16,
        }
    }
}

#[derive(Debug, Default)]
struct AtomicField {
    key: AtomicU64,
    data: AtomicU64,
}

impl AtomicField {
    fn read(&self, hash: ZHash) -> Option<TTEntry> {
        let checksum = hash.0;
        let key = self.key.load(Ordering::SeqCst);
        let data = self.data.load(Ordering::SeqCst);
        if key ^ checksum == data {
            Some(TTEntry::from((key, data)))
        } else {
            None
        }
    }

    fn read_unchecked(&self) -> TTEntry {
        let key = self.key.load(Ordering::SeqCst);
        let data = self.data.load(Ordering::SeqCst);
        TTEntry::from((key, data))
    }

    fn write(&self, entry: TTEntry) {
        let (key, data) = entry.into();
        self.key.store(key, Ordering::SeqCst);
        self.data.store(data, Ordering::SeqCst);
    }
}

pub struct TranspositionTable {
    table: Vec<AtomicField>,
    age: u8,
}

impl TranspositionTable {
    pub const DEFAULT_SIZE: usize = 32; // 32 MB

    pub fn new(mb_size: usize) -> Self {
        let mut tt = Self {
            table: Vec::new(),
            age: 0,
        };
        tt.resize(mb_size);
        tt
    }

    pub fn get_key(&self, hash: ZHash) -> usize {
        let key = hash.0 as u128;
        let len = self.table.len() as u128;
        ((key * len) >> 64) as usize
    }

    pub fn resize(&mut self, mb_size: usize) {
        let new_size = (mb_size * 1024 * 1024) / std::mem::size_of::<AtomicField>();
        self.table.resize_with(new_size, AtomicField::default);
    }

    pub fn clear(&mut self) {
        self.age = 0;
        self.table.iter().for_each(|entry| {
            entry.key.store(0, Ordering::SeqCst);
            entry.data.store(0, Ordering::SeqCst);
        });
    }

    pub fn increment_age(&mut self) {
        self.age = (self.age + 1) & 0b01111111;
    }

    pub fn probe(&self, hash: ZHash) -> Option<TTEntry> {
        unsafe { self.table.get_unchecked(self.get_key(hash)).read(hash) }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert(
        &self,
        hash: ZHash,
        flag: TTFlag,
        best_move: Move,
        eval: i32,
        value: i32,
        depth: usize,
        ply: usize,
    ) {
        let old_slot = unsafe { self.table.get_unchecked(self.get_key(hash)) };
        let old = old_slot.read_unchecked();
        let same_position = hash.0 == old.key;

        if self.age != old.age
            || !same_position
            || flag == TTFlag::Exact
            || depth + 2 > old.depth as usize
        {
            let tt_value = if value >= LONGEST_TB_MATE {
                (value + ply as i32) as i16
            } else if value <= -LONGEST_TB_MATE {
                (value - ply as i32) as i16
            } else {
                value as i16
            };

            old_slot.write(TTEntry {
                key: hash.0,
                age: self.age,
                depth: depth as u8,
                flag,
                best_move,
                eval: eval as i16,
                value: tt_value,
            });
        }
    }
}
