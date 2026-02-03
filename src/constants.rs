use crate::bitboard::BitBoard;

use super::square::Square;

pub const PIECE_VALUES: [i32; 6] = [
    98,   // Pawn
    435,  // Knight
    422,  // Bishop
    593,  // Rook
    1011, // Queen
    0,    // King
];

const fn make_between_table() -> [[BitBoard; 64]; 64] {
    let mut table = [[BitBoard::EMPTY; 64]; 64];
    let mut i = 0;
    while i < 64 {
        let mut j = 0;
        while j < 64 {
            let sq1 = Square::new(i as u8);
            let sq2 = Square::new(j as u8);

            table[i][j] = if rook_attacks(BitBoard::EMPTY.0, sq1.index()).contains(sq2) {
                rook_attacks(sq2.to_board().0, sq1.index())
                    .and(rook_attacks(sq1.to_board().0, sq2.index()))
            } else if bishop_attacks(BitBoard::EMPTY.0, sq1.index()).contains(sq2) {
                bishop_attacks(sq2.to_board().0, sq1.index())
                    .and(bishop_attacks(sq1.to_board().0, sq2.index()))
            } else {
                BitBoard::EMPTY
            };

            j += 1;
        }
        i += 1;
    }
    table
}

pub static BETWEEN: [[BitBoard; 64]; 64] = make_between_table();

pub const fn between(sq1: Square, sq2: Square) -> BitBoard {
    BETWEEN[sq1.index()][sq2.index()]
}

const fn make_pinned_moves_table() -> [[BitBoard; 64]; 64] {
    let mut table = [[BitBoard::EMPTY; 64]; 64];
    let mut king_idx = 0;
    while king_idx < 64 {
        let mut pinned_idx = 0;
        while pinned_idx < 64 {
            let king = Square::new(king_idx as u8);
            let pinned = Square::new(pinned_idx as u8);

            table[king_idx][pinned_idx] =
                if bishop_attacks(BitBoard::EMPTY.0, pinned.index()).contains(king) {
                    bishop_attacks(BitBoard::EMPTY.0, king.index())
                        .and(bishop_attacks(king.to_board().0, pinned.index()))
                } else if rook_attacks(BitBoard::EMPTY.0, pinned.index()).contains(king) {
                    rook_attacks(BitBoard::EMPTY.0, king.index())
                        .and(rook_attacks(king.to_board().0, pinned.index()))
                } else {
                    BitBoard::EMPTY
                };

            pinned_idx += 1;
        }
        king_idx += 1;
    }
    table
}

pub static PINNED_MOVES: [[BitBoard; 64]; 64] = make_pinned_moves_table();

pub const fn pinned_moves(king_sq: Square, pinned: Square) -> BitBoard {
    PINNED_MOVES[king_sq.index()][pinned.index()]
}

#[derive(Clone, Copy, Debug)]
struct SMasks {
    pub lower: u64,
    pub upper: u64,
    pub line_ex: u64,
}

impl SMasks {
    pub const fn new(lower: u64, upper: u64) -> Self {
        let line_ex = lower | upper;
        SMasks {
            lower,
            upper,
            line_ex,
        }
    }
}

const fn line_attacks(occ: u64, mask: &SMasks) -> u64 {
    let lower: u64 = mask.lower & occ;
    let upper: u64 = mask.upper & occ;
    let ms1_b = 0x8000000000000000 >> (lower | 1).leading_zeros();
    let odiff = upper ^ upper.wrapping_sub(ms1_b);
    mask.line_ex & odiff
}

pub const fn rook_attacks(occ: u64, sq: usize) -> BitBoard {
    BitBoard(line_attacks(occ, &MASKS[sq][0]) | line_attacks(occ, &MASKS[sq][1]))
}

pub const fn bishop_attacks(occ: u64, sq: usize) -> BitBoard {
    BitBoard(line_attacks(occ, &MASKS[sq][2]) | line_attacks(occ, &MASKS[sq][3]))
}

pub fn queen_attacks(occ: u64, sq: usize) -> BitBoard {
    rook_attacks(occ, sq) | bishop_attacks(occ, sq)
}

pub const CASTLE: [[Square; 2]; 2] = [
    [Square::new(2), Square::new(6)],   // c1 g1
    [Square::new(58), Square::new(62)], // c8 f8
];

const FILE_A: u64 = 0x0101010101010101;
const FILE_B: u64 = FILE_A << 1;
const FILE_G: u64 = FILE_A << 6;
const FILE_H: u64 = FILE_A << 7;

pub const PAWN_ATTACKS: [[BitBoard; 64]; 2] = {
    let mut attacks = [[BitBoard(0); 64]; 2];
    let mut i = 0;
    while i < 64 {
        let bit = 1u64 << i;
        let mut w = 0;
        if (bit & FILE_A) == 0 {
            w |= bit << 7;
        }
        if (bit & FILE_H) == 0 {
            w |= bit << 9;
        }
        attacks[0][i] = BitBoard(w);

        let mut b = 0;
        if (bit & FILE_A) == 0 {
            b |= bit >> 9;
        }
        if (bit & FILE_H) == 0 {
            b |= bit >> 7;
        }
        attacks[1][i] = BitBoard(b);

        i += 1;
    }
    attacks
};

pub const KNIGHT_ATTACKS: [BitBoard; 64] = {
    let mut attacks = [BitBoard(0); 64];
    let mut i = 0;
    while i < 64 {
        let bit = 1u64 << i;
        let mut att = 0;
        if (bit & FILE_A) == 0 {
            att |= (bit << 15) | (bit >> 17);
        }
        if (bit & FILE_H) == 0 {
            att |= (bit << 17) | (bit >> 15);
        }
        if (bit & (FILE_A | FILE_B)) == 0 {
            att |= (bit << 6) | (bit >> 10);
        }
        if (bit & (FILE_G | FILE_H)) == 0 {
            att |= (bit << 10) | (bit >> 6);
        }
        attacks[i] = BitBoard(att);
        i += 1;
    }
    attacks
};

pub const KING_ATTACKS: [BitBoard; 64] = {
    let mut attacks = [BitBoard(0); 64];
    let mut i = 0;
    while i < 64 {
        let bit = 1u64 << i;
        let mut att = 0;
        att |= (bit << 8) | (bit >> 8);
        if (bit & FILE_H) == 0 {
            att |= (bit << 1) | (bit << 9) | (bit >> 7);
        }
        if (bit & FILE_A) == 0 {
            att |= (bit >> 1) | (bit << 7) | (bit >> 9);
        }
        attacks[i] = BitBoard(att);
        i += 1;
    }
    attacks
};

const MASKS: [[SMasks; 4]; 64] = {
    let mut table = [[SMasks {
        lower: 0,
        upper: 0,
        line_ex: 0,
    }; 4]; 64];

    let mut sq = 0;
    while sq < 64 {
        let r = (sq / 8) as i8;
        let f = (sq % 8) as i8;

        let dirs = [
            ([-1, 0], [1, 0]),
            ([0, -1], [0, 1]),
            ([-1, -1], [1, 1]),
            ([1, -1], [-1, 1]),
        ];

        let mut i = 0;
        while i < 4 {
            let (d_lo, d_hi) = dirs[i];

            // Lower Ray
            let mut lower = 0u64;
            let mut cf = f + d_lo[0];
            let mut cr = r + d_lo[1];
            while cf >= 0 && cf <= 7 && cr >= 0 && cr <= 7 {
                lower |= 1u64 << (cr * 8 + cf);
                cf += d_lo[0];
                cr += d_lo[1];
            }

            // Upper Ray
            let mut upper = 0u64;
            let mut cf = f + d_hi[0];
            let mut cr = r + d_hi[1];
            while cf >= 0 && cf <= 7 && cr >= 0 && cr <= 7 {
                upper |= 1u64 << (cr * 8 + cf);
                cf += d_hi[0];
                cr += d_hi[1];
            }

            table[sq][i] = SMasks::new(lower, upper);
            i += 1;
        }
        sq += 1;
    }
    table
};

const fn xorshift64star(mut s: u64) -> u64 {
    s ^= s >> 12;
    s ^= s << 25;
    s ^= s >> 27;
    s.wrapping_mul(0x2545F4914F6CDD1D)
}

const SEED: u64 = 2361912;

pub const PIECE_KEYS: [[u64; 64]; 12] = {
    let mut table = [[0; 64]; 12];
    let mut s = SEED;

    let mut i = 0;
    while i < 50 {
        s = xorshift64star(s);
        i += 1;
    }

    let mut p = 0;
    while p < 12 {
        let mut sq = 0;
        while sq < 64 {
            s = xorshift64star(s);
            table[p][sq] = s;
            sq += 1;
        }
        p += 1;
    }
    table
};

pub const EP_KEYS: [u64; 64] = {
    let mut table = [0; 64];
    let mut s = xorshift64star(SEED ^ 0x9E3779B97F4A7C15);
    let mut i = 0;
    while i < 64 {
        s = xorshift64star(s);
        table[i] = s;
        i += 1;
    }
    table
};

pub const CASTLE_KEYS: [u64; 16] = {
    let mut table = [0; 16];
    let mut s = xorshift64star(SEED ^ 0xBF58476D1CE4E5B9);
    let mut i = 0;
    while i < 16 {
        s = xorshift64star(s);
        table[i] = s;
        i += 1;
    }
    table
};

pub const SIDE_KEY: u64 = {
    let s = xorshift64star(SEED ^ 0x55AA55AA55AA55AA);
    xorshift64star(s)
};
