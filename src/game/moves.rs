use crate::game::square::Square;

use super::{
    bitboard::BitBoard,
    board::Board,
    constants::{KING_ATTACKS, KNIGHT_ATTACKS},
    piece::{Colour, Piece},
};

/// A move needs 16 bits to be stored, the information is contained
/// in the following way:
///
/// bits [0-5]: Origin square (2^6 = 64 possible positions)
/// bits [6-11]: Destination square (2^6 = 64 possible positions)
/// bits [12-13]: Promotion piece type (Knight|Rook|Queen|Bishop)
/// bits [14-15]: If the move is a promotion, an en passant move or castling
#[derive(PartialEq, Eq, PartialOrd, Clone, Copy, Debug, Default, Hash, Ord)]
pub struct Move(pub u16);

const SRC: u16 = 0b0000_0000_0011_1111;
const DST: u16 = 0b0000_1111_1100_0000;
const TYPE: u16 = 0b1111_0000_0000_0000;

impl Move {
    pub const NULL: Self = Self(0);

    pub fn new(src: Square, dest: Square, kind: MoveKind) -> Self {
        Self((src.index() as u16) | ((dest.index() as u16) << 6) | ((kind as u16) << 12))
    }

    pub const fn get_source(self) -> Square {
        Square::new((self.0 & SRC) as usize)
    }

    pub const fn get_dest(self) -> Square {
        Square::new(((self.0 & DST) >> 6) as usize)
    }

    pub fn get_type(self) -> MoveKind {
        match (self.0 & TYPE) >> 12 {
            0b0000 => MoveKind::Quiet,
            0b0001 => MoveKind::Castle,
            0b0010 => MoveKind::DoublePush,
            0b1000 => MoveKind::Capture,
            0b1001 => MoveKind::EnPassant,

            0b0100 => MoveKind::KnightPromotion,
            0b0101 => MoveKind::BishopPromotion,
            0b0110 => MoveKind::RookPromotion,
            0b0111 => MoveKind::QueenPromotion,

            0b1100 => MoveKind::KnightCapPromo,
            0b1101 => MoveKind::BishopCapPromo,
            0b1110 => MoveKind::RookCapPromo,
            0b1111 => MoveKind::QueenCapPromo,

            _ => unreachable!(),
        }
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!("{}{}", self.get_source(), self.get_dest());
        let move_type = self.get_type();

        if move_type.is_promotion() {
            write!(
                f,
                "{}{}",
                s,
                move_type.get_promotion(Colour::Black).to_char()
            )
        } else {
            write!(f, "{s}")
        }
    }
}
/// MoveKind is a 4-bit enum that represents the type of move
/// Structured as follows:
///
/// 4rd bit: 1 if the move is a promotion, 0 otherwise
#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash)]
pub enum MoveKind {
    Quiet = 0b0000,
    Castle = 0b0001,
    DoublePush = 0b0010,

    // Promotions have 3rd bit set
    KnightPromotion = 0b0100,
    BishopPromotion = 0b0101,
    RookPromotion = 0b0110,
    QueenPromotion = 0b0111,

    // Captures have 4th bit set
    Capture = 0b1000,
    EnPassant = 0b1001,

    KnightCapPromo = 0b1100,
    BishopCapPromo = 0b1101,
    RookCapPromo = 0b1110,
    QueenCapPromo = 0b1111,
}

impl MoveKind {
    pub const fn is_promotion(self) -> bool {
        self as usize & 0b0100 != 0
    }

    pub const fn is_capture(self) -> bool {
        self as usize & 0b1000 != 0
    }

    pub const fn get_promotion(self, side: Colour) -> Piece {
        const PROMO_MASK: usize = 0b0011;
        const PROMO_PIECES: [[Piece; 4]; 2] = [
            [Piece::WN, Piece::WB, Piece::WR, Piece::WQ],
            [Piece::BN, Piece::BB, Piece::BR, Piece::BQ],
        ];

        PROMO_PIECES[side as usize][self as usize & PROMO_MASK]
    }
}

/// Move Generation Logic
impl Board {
    pub fn all_slider_moves<const QUIET: bool>(
        &self,
        src: Square,
        occ: u64,
        attacks_fn: fn(u64, usize) -> BitBoard,
        moves: &mut MoveList,
    ) {
        let attacks = attacks_fn(occ, src.index());

        if QUIET {
            let mut quiets = attacks & !BitBoard(occ);
            while quiets != BitBoard::EMPTY {
                let dst = quiets.lsb();
                moves.push(Move::new(src, dst, MoveKind::Quiet));
                quiets = quiets.pop_bit(dst);
            }
        }

        let mut caps = attacks & self.sides[!self.side as usize];
        while caps != BitBoard::EMPTY {
            let dst = caps.lsb();
            moves.push(Move::new(src, dst, MoveKind::Capture));
            caps = caps.pop_bit(dst);
        }
    }

    pub fn all_king_moves<const QUIET: bool>(&self, src: Square, occ: u64, moves: &mut MoveList) {
        let attacks = KING_ATTACKS[src.index()];

        if QUIET {
            let mut quiets = attacks & !BitBoard(occ);
            while quiets != BitBoard::EMPTY {
                let dst = quiets.lsb();
                moves.push(Move::new(src, dst, MoveKind::Quiet));
                quiets = quiets.pop_bit(dst);
            }

            if src.to_board() & BitBoard::KING_START_POS != BitBoard::EMPTY {
                for &dest in &[
                    Square::from("g1"),
                    Square::from("c1"),
                    Square::from("g8"),
                    Square::from("c8"),
                ] {
                    if self.is_castle_legal(dest) {
                        moves.push(Move::new(src, dest, MoveKind::Castle));
                    }
                }
            }
        }

        let mut caps = attacks & self.sides[!self.side as usize];
        while caps != BitBoard::EMPTY {
            let dst = caps.lsb();
            moves.push(Move::new(src, dst, MoveKind::Capture));
            caps = caps.pop_bit(dst);
        }
    }

    pub fn all_knight_moves<const QUIET: bool>(&self, src: Square, occ: u64, moves: &mut MoveList) {
        let attacks = KNIGHT_ATTACKS[src.index()];

        if QUIET {
            let mut quiets = attacks & !BitBoard(occ);
            while quiets != BitBoard::EMPTY {
                let dst = quiets.lsb();
                moves.push(Move::new(src, dst, MoveKind::Quiet));
                quiets = quiets.pop_bit(dst);
            }
        }

        let mut caps = attacks & self.sides[!self.side as usize];
        while caps != BitBoard::EMPTY {
            let dst = caps.lsb();
            moves.push(Move::new(src, dst, MoveKind::Capture));
            caps = caps.pop_bit(dst);
        }
    }

    pub fn all_pawn_moves<const QUIET: bool>(
        &self,
        src: Square,
        colour: Colour,
        occ: BitBoard,
        moves: &mut MoveList,
    ) {
        let forward = colour.forward();
        let start_rank = BitBoard::START_RANKS[colour as usize];
        let promo_rank = BitBoard::PROMO_RANKS[colour as usize];
        let opponent = self.sides[!colour as usize];

        if let Some(dest) = src.jump(0, forward) {
            if !occ.get_bit(dest) {
                if promo_rank.get_bit(dest) {
                    moves.push(Move::new(src, dest, MoveKind::QueenPromotion));
                    moves.push(Move::new(src, dest, MoveKind::RookPromotion));
                    moves.push(Move::new(src, dest, MoveKind::BishopPromotion));
                    moves.push(Move::new(src, dest, MoveKind::KnightPromotion));
                } else if QUIET {
                    moves.push(Move::new(src, dest, MoveKind::Quiet));
                    if start_rank.get_bit(src) {
                        let dbl = src.jump(0, 2 * forward).unwrap();
                        if !occ.get_bit(dbl) {
                            moves.push(Move::new(src, dbl, MoveKind::DoublePush));
                        }
                    }
                }
            }
        }

        for delta in [(-1, forward), (1, forward)] {
            if let Some(dest) = src.jump(delta.0, delta.1) {
                if opponent.get_bit(dest) {
                    if promo_rank.get_bit(dest) {
                        moves.push(Move::new(src, dest, MoveKind::QueenCapPromo));
                        moves.push(Move::new(src, dest, MoveKind::RookCapPromo));
                        moves.push(Move::new(src, dest, MoveKind::BishopCapPromo));
                        moves.push(Move::new(src, dest, MoveKind::KnightCapPromo));
                    } else {
                        moves.push(Move::new(src, dest, MoveKind::Capture));
                    }
                } else if self.en_passant == Some(dest) {
                    let ep_target = dest.jump(0, -forward).expect("Invalid en passant target");
                    if opponent.get_bit(ep_target) {
                        moves.push(Move::new(src, dest, MoveKind::EnPassant));
                    }
                }
            }
        }
    }
}

pub struct MoveList {
    pub moves: [Move; MoveList::SIZE],
    len: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveList {
    // Pointer width 64
    pub const SIZE: usize = 252;

    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            moves: [Move::NULL; Self::SIZE],
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [Move] {
        &mut self.moves[..self.len]
    }
    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push(&mut self, m: Move) {
        self.moves[self.len] = m;
        self.len += 1;
    }

    pub fn pick(&mut self, scores: &mut [i32; MoveList::SIZE]) -> Option<(Move, i32)> {
        if self.len == 0 {
            return None;
        }

        let (mut best_idx, mut best_score) = (0, i32::MIN);

        if let Some((i, &score)) = (0..self.len).zip(scores.iter()).max_by_key(|&(_, &s)| s) {
            best_score = score;
            best_idx = i;
        }

        self.len -= 1;
        self.moves.swap(best_idx, self.len);
        scores.swap(best_idx, self.len);

        Some((self.moves[self.len], best_score))
    }
}

impl IntoIterator for MoveList {
    type Item = Move;
    type IntoIter = core::iter::Take<core::array::IntoIter<Move, { MoveList::SIZE }>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.moves.into_iter().take(self.len)
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = Move;
    type IntoIter = core::iter::Copied<core::slice::Iter<'a, Move>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.moves[..self.len].iter().copied()
    }
}
