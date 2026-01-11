use std::arch::x86_64::*;

// Square: 0-63
// Piece: Pawn = 0, Knight = 1, Bishop = 2, Rook = 3, Queen = 4, King = 5
// Side: White = 0, Black = 1
// Network (768x8 -> 1536)x2 -> 1
const INPUT_SIZE: usize = 768;
pub const HL_SIZE: usize = 1536;

// Quantization factors
const QA: i32 = 255;
const QB: i32 = 64;
const QAB: i32 = QA * QB;

// Scaling factor
const SCALE: i32 = 400;
pub const NUM_BUCKETS: usize = 8;

#[rustfmt::skip]
pub static BUCKETS: [usize; 64] = [
    0, 1, 2, 3, 11, 10, 9, 8,
    4, 4, 5, 5, 13, 13, 12, 12,
    6, 6, 6, 6, 14, 14, 14, 14,
    6, 6, 6, 6, 14, 14, 14, 14,
    7, 7, 7, 7, 15, 15, 15, 15,
    7, 7, 7, 7, 15, 15, 15, 15,
    7, 7, 7, 7, 15, 15, 15, 15,
    7, 7, 7, 7, 15, 15, 15, 15,
];

pub static NNUE: Network =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/oxide-v4.bin")) };

#[repr(C)]
pub struct Network {
    pub feature_weights: [Accumulator; INPUT_SIZE * NUM_BUCKETS],
    pub feature_bias: Accumulator,
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

    #[inline]
    #[rustfmt::skip]
    pub const fn get_bucket<const SIDE: usize>(king_sq: usize) -> usize {
        BUCKETS[if SIDE == 1 { king_sq ^ 0b111000 } else { king_sq }]
    }

    pub const fn get_base_index<const SIDE: usize>(
        side: usize,
        pc: usize,
        mut king_sq: usize,
    ) -> usize {
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
    #[cfg(not(target_feature = "avx512f"))]
    #[inline]
    pub fn update_multi(&mut self, adds: &[u16], subs: &[u16]) {
        const REGS: usize = 8;
        const PER: usize = 128;
        const ITERATIONS: usize = HL_SIZE / PER;

        unsafe {
            for i in 0..ITERATIONS {
                let offset = i * PER;
                let mut regs = [_mm256_setzero_si256(); REGS];

                for (j, reg) in regs.iter_mut().enumerate() {
                    *reg = _mm256_load_si256(self.vals.as_ptr().add(offset + j * 16).cast());
                }

                for &add in adds {
                    let weights = NNUE.feature_weights[add as usize].vals.as_ptr().add(offset);
                    for (j, reg) in regs.iter_mut().enumerate() {
                        let w = _mm256_load_si256(weights.add(j * 16).cast());
                        *reg = _mm256_add_epi16(*reg, w);
                    }
                }

                for &sub in subs {
                    let weights = NNUE.feature_weights[sub as usize].vals.as_ptr().add(offset);
                    for (j, reg) in regs.iter_mut().enumerate() {
                        let w = _mm256_load_si256(weights.add(j * 16).cast());
                        *reg = _mm256_sub_epi16(*reg, w);
                    }
                }

                for (j, reg) in regs.iter().enumerate() {
                    _mm256_store_si256(self.vals.as_mut_ptr().add(offset + j * 16).cast(), *reg);
                }
            }
        }
    }

    #[cfg(target_feature = "avx512f")]
    #[inline]
    pub fn update_multi(&mut self, adds: &[u16], subs: &[u16]) {
        const REGS: usize = 8;
        const PER: usize = 256;
        const ITERATIONS: usize = HL_SIZE / PER;

        unsafe {
            for i in 0..ITERATIONS {
                let offset = i * PER;
                let mut regs = [_mm512_setzero_si512(); REGS];

                for (j, reg) in regs.iter_mut().enumerate() {
                    *reg = _mm512_load_si512(self.vals.as_ptr().add(offset + j * 32).cast());
                }

                for &add in adds {
                    let weights = NNUE.feature_weights[add as usize].vals.as_ptr().add(offset);
                    for (j, reg) in regs.iter_mut().enumerate() {
                        let w = _mm512_load_si512(weights.add(j * 32).cast());
                        *reg = _mm512_add_epi16(*reg, w);
                    }
                }

                for &sub in subs {
                    let weights = NNUE.feature_weights[sub as usize].vals.as_ptr().add(offset);
                    for (j, reg) in regs.iter_mut().enumerate() {
                        let w = _mm512_load_si512(weights.add(j * 32).cast());
                        *reg = _mm512_sub_epi16(*reg, w);
                    }
                }

                for (j, reg) in regs.iter().enumerate() {
                    _mm512_store_si512(self.vals.as_mut_ptr().add(offset + j * 32).cast(), *reg);
                }
            }
        }
    }
}

impl Default for Accumulator {
    fn default() -> Self {
        NNUE.feature_bias
    }
}

#[derive(Clone, Copy)]
pub struct EvalEntry {
    pub bbs: [u64; 8], // Bitboards for pieces and sides
    pub white: Accumulator,
    pub black: Accumulator,
}

pub struct EvalTable {
    pub table: Box<[[EvalEntry; 2 * NUM_BUCKETS]; 2 * NUM_BUCKETS]>,
}

impl Default for EvalTable {
    fn default() -> Self {
        let bias = NNUE.feature_bias;
        let entry = EvalEntry {
            bbs: [0; 8],
            white: bias,
            black: bias,
        };
        Self {
            table: Box::new([[entry; 2 * NUM_BUCKETS]; 2 * NUM_BUCKETS]),
        }
    }
}

#[cfg(not(target_feature = "avx512vnni"))]
#[inline]
pub unsafe fn flatten(acc: &Accumulator, weights: &Accumulator) -> i32 {
    const CHUNK: usize = 16;
    const NUM_ITERS: usize = HL_SIZE / CHUNK;

    let mut sum = _mm256_setzero_si256();
    let min = _mm256_setzero_si256();
    let max = _mm256_set1_epi16(QA as i16);

    for i in 0..NUM_ITERS {
        let mut v = load_i16s(acc, i * CHUNK);
        v = _mm256_min_epi16(_mm256_max_epi16(v, min), max);
        let w = load_i16s(weights, i * CHUNK);
        let product = _mm256_madd_epi16(v, _mm256_mullo_epi16(v, w));
        sum = _mm256_add_epi32(sum, product);
    }

    horizontal_sum_i32(sum)
}

#[cfg(not(target_feature = "avx512vnni"))]
#[inline]
unsafe fn load_i16s(acc: &Accumulator, start_idx: usize) -> __m256i {
    _mm256_load_si256(acc.vals.as_ptr().add(start_idx).cast())
}

#[cfg(not(target_feature = "avx512vnni"))]
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

#[cfg(target_feature = "avx512vnni")]
#[inline]
pub unsafe fn flatten(acc: &Accumulator, weights: &Accumulator) -> i32 {
    const CHUNK: usize = 32;
    const NUM_ITERS: usize = HL_SIZE / CHUNK;

    let mut sum = _mm512_setzero_si512();
    let min = _mm512_setzero_si512();
    let max = _mm512_set1_epi16(QA as i16);

    for i in 0..NUM_ITERS {
        let mut v = load_i16s(acc, i * CHUNK);
        v = _mm512_min_epi16(_mm512_max_epi16(v, min), max);
        let w = load_i16s(weights, i * CHUNK);
        let product = _mm512_mullo_epi16(v, w);
        sum = _mm512_dpwssd_epi32(sum, v, product);
    }

    _mm512_reduce_add_epi32(sum)
}

#[cfg(target_feature = "avx512vnni")]
#[inline]
unsafe fn load_i16s(acc: &Accumulator, start_idx: usize) -> __m512i {
    _mm512_load_si512(acc.vals.as_ptr().add(start_idx).cast())
}
