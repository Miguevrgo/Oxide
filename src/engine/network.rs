use crate::game::{
    piece::{Colour, Piece},
    square::Square,
};
use std::arch::x86_64::*;

// Square: 0-63
// Piece: Pawn = 0, Knight = 1, Bishop = 2, Rook = 3, Queen = 4, King = 5
// Side: White = 0, Black = 1
const INPUT_SIZE: usize = 768;
pub const HL_SIZE: usize = 1024;

const SCALE: i32 = 400;
const QA: i32 = 255;
const QB: i32 = 64;
const QAB: i32 = QA * QB;
const NUM_BUCKETS: usize = 4;

#[rustfmt::skip]
static BUCKETS: [usize; 64] = [
    0, 0, 1, 1, 5, 5, 4, 4,
    2, 2, 2, 2, 6, 6, 6, 6,
    3, 3, 3, 3, 7, 7, 7, 7,
    3, 3, 3, 3, 7, 7, 7, 7,
    3, 3, 3, 3, 7, 7, 7, 7,
    3, 3, 3, 3, 7, 7, 7, 7,
    3, 3, 3, 3, 7, 7, 7, 7,
    3, 3, 3, 3, 7, 7, 7, 7,
];

pub static NNUE: Network =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/nnue-net.bin")) };

#[repr(C)]
pub struct Network {
    pub accumulator_weights: [Accumulator; INPUT_SIZE * NUM_BUCKETS],
    pub accumulator_biases: Accumulator,
    output_weights: [Accumulator; 2],
    output_bias: i16,
}

impl Network {
    pub fn out(boys: &Accumulator, opps: &Accumulator) -> i32 {
        let weights = &NNUE.output_weights;
        unsafe {
            let sum = flatten(boys, &weights[0]) + flatten(opps, &weights[1]);
            (sum / QA + i32::from(NNUE.output_bias)) * SCALE / QAB
        }
    }

    pub fn get_bucket<const SIDE: usize>(mut king_sq: usize) -> usize {
        if SIDE == 1 {
            king_sq ^= 0b111000;
        }

        BUCKETS[king_sq]
    }

    pub fn get_base_index<const SIDE: usize>(side: usize, pc: usize, mut king_sq: usize) -> usize {
        if king_sq % 8 > 3 {
            king_sq ^= 7;
        }

        if SIDE == 0 {
            INPUT_SIZE * Self::get_bucket::<0>(king_sq) + [0, 384][side] + 64 * pc
        } else {
            INPUT_SIZE * Self::get_bucket::<1>(king_sq) + [384, 0][side] + 64 * pc
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
pub struct Accumulator {
    vals: [i16; HL_SIZE],
}

impl Accumulator {
    pub fn update_multi(&mut self, adds: &[u16]) {
        const REGS: usize = 8;
        const PER: usize = REGS * 16;

        let mut regs = [0i16; PER];

        for i in 0..HL_SIZE / PER {
            let offset = PER * i;

            for (j, reg) in regs.iter_mut().enumerate() {
                *reg = self.vals[offset + j];
            }

            for &add in adds {
                let weights = &NNUE.accumulator_weights[usize::from(add)];

                for (j, reg) in regs.iter_mut().enumerate() {
                    *reg += weights.vals[offset + j];
                }
            }

            for (j, reg) in regs.iter().enumerate() {
                self.vals[offset + j] = *reg;
            }
        }
    }
}

impl Default for Accumulator {
    fn default() -> Self {
        NNUE.accumulator_biases
    }
}

pub struct EvalEntry {
    pub bbs: [u64; 8],
    pub white: Accumulator,
    pub black: Accumulator,
}

pub struct EvalTable {
    pub table: Box<[[EvalEntry; 2 * NUM_BUCKETS]; 2 * NUM_BUCKETS]>,
}

impl Default for EvalTable {
    fn default() -> Self {
        let mut table: Box<[[EvalEntry; 2 * NUM_BUCKETS]; 2 * NUM_BUCKETS]> =
            unsafe { boxed_and_zeroed() };

        for row in table.iter_mut() {
            for entry in row.iter_mut() {
                entry.white = Accumulator::default();
                entry.black = Accumulator::default();
            }
        }

        Self { table }
    }
}

pub fn screlu(x: i16) -> i32 {
    i32::from(x.clamp(0, QA as i16)).pow(2)
}

pub unsafe fn flatten(acc: &Accumulator, weights: &Accumulator) -> i32 {
    const CHUNK: usize = 16;

    let mut sum = _mm256_setzero_si256();
    let min = _mm256_setzero_si256();
    let max = _mm256_set1_epi16(QA as i16);

    for i in 0..HL_SIZE / CHUNK {
        let mut v = load_i16s(acc, i * CHUNK);
        v = _mm256_min_epi16(_mm256_max_epi16(v, min), max);
        let w = load_i16s(weights, i * CHUNK);
        let product = _mm256_madd_epi16(v, _mm256_mullo_epi16(v, w));
        sum = _mm256_add_epi32(sum, product);
    }

    horizontal_sum_i32(sum)
}

#[inline]
unsafe fn load_i16s(acc: &Accumulator, start_idx: usize) -> __m256i {
    _mm256_load_si256(acc.vals.as_ptr().add(start_idx).cast())
}

#[inline]
unsafe fn horizontal_sum_i32(sum: __m256i) -> i32 {
    let upper_128 = _mm256_extracti128_si256::<1>(sum);
    let lower_128 = _mm256_castsi256_si128(sum);
    let sum_128 = _mm_add_epi32(upper_128, lower_128);
    let upper_64 = _mm_unpackhi_epi64(sum_128, sum_128);
    let sum_64 = _mm_add_epi32(upper_64, sum_128);
    let upper_32 = _mm_shuffle_epi32::<0b00_00_00_01>(sum_64);
    let sum_32 = _mm_add_epi32(upper_32, sum_64);

    _mm_cvtsi128_si32(sum_32)
}

pub unsafe fn boxed_and_zeroed<T>() -> Box<T> {
    let layout = std::alloc::Layout::new::<T>();
    let ptr = std::alloc::alloc_zeroed(layout);
    if ptr.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    Box::from_raw(ptr.cast())
}
