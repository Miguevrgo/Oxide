use crate::game::square::Square;

use super::{
    bitboard::BitBoard,
    board::Board,
    piece::{Colour, Piece},
};

#[rustfmt::skip]
const KNIGHT_OFFSETS: [(i8, i8); 8] = [
    (2, 1), (2, -1), (-2, 1), (-2, -1),
    (1, 2), (1, -2), (-1, 2), (-1, -2),
];
#[rustfmt::skip]
const KING_OFFSETS: [(i8, i8); 8] = [
    (1, 0), (1, 1), (1, -1), (0, 1),
    (0, -1), (-1, 0), (-1, 1), (-1, -1),
];
const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
const ROOK_DIRECTIONS: [(i8, i8); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

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
    pub fn new(src: Square, dest: Square, kind: MoveKind) -> Self {
        Self((src.index() as u16) | ((dest.index() as u16) << 6) | ((kind as u16) << 12))
    }

    pub fn get_source(self) -> Square {
        Square::new((self.0 & SRC) as usize)
    }

    pub fn get_dest(self) -> Square {
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

pub fn all_pawn_moves(src: Square, piece: Piece, board: &Board) -> Vec<Move> {
    let mut moves = Vec::with_capacity(4);
    let forward = piece.colour().forward();

    let start_rank = BitBoard::START_RANKS[piece.colour() as usize];
    let promo_rank = BitBoard::PROMO_RANKS[piece.colour() as usize];
    let opponent = board.sides[!piece.colour() as usize];

    if let Some(dest) = src.jump(0, forward) {
        if promo_rank.get_bit(dest) {
            moves.push(Move::new(src, dest, MoveKind::QueenPromotion));
            moves.push(Move::new(src, dest, MoveKind::RookPromotion));
            moves.push(Move::new(src, dest, MoveKind::BishopPromotion));
            moves.push(Move::new(src, dest, MoveKind::KnightPromotion));
        } else {
            moves.push(Move::new(src, dest, MoveKind::Quiet));
        }
    }

    if start_rank.get_bit(src) {
        moves.push(Move::new(
            src,
            src.jump(0, 2 * forward).unwrap(),
            MoveKind::DoublePush,
        ));
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
            } else if board.en_passant == Some(dest) {
                let ep_target = dest.jump(0, -forward).expect("Invalid en passant target");
                if opponent.get_bit(ep_target) {
                    moves.push(Move::new(src, dest, MoveKind::EnPassant));
                }
            }
        }
    }
    moves
}

pub fn all_knight_moves(src: Square) -> Vec<Move> {
    let mut moves = Vec::with_capacity(8);

    for &(file_delta, rank_delta) in &KNIGHT_OFFSETS {
        if let Some(dest) = src.jump(file_delta, rank_delta) {
            moves.push(Move::new(src, dest, MoveKind::Quiet));
            moves.push(Move::new(src, dest, MoveKind::Capture));
        }
    }

    moves
}

fn sliding_moves(src: Square, board: &Board, directions: &[(i8, i8)]) -> Vec<Move> {
    let mut moves = Vec::with_capacity(8);
    for &(file_delta, rank_delta) in directions {
        let mut dest = src;
        while let Some(next) = dest.jump(file_delta, rank_delta) {
            dest = next;
            if board.piece_at(dest).is_some() {
                moves.push(Move::new(src, dest, MoveKind::Capture));
                break;
            }
            moves.push(Move::new(src, dest, MoveKind::Quiet));
        }
    }
    moves
}

pub fn all_bishop_moves(src: Square, board: &Board) -> Vec<Move> {
    sliding_moves(src, board, &BISHOP_DIRECTIONS)
}

pub fn all_rook_moves(src: Square, board: &Board) -> Vec<Move> {
    sliding_moves(src, board, &ROOK_DIRECTIONS)
}

pub fn all_queen_moves(src: Square, board: &Board) -> Vec<Move> {
    sliding_moves(src, board, &BISHOP_DIRECTIONS)
        .into_iter()
        .chain(sliding_moves(src, board, &ROOK_DIRECTIONS))
        .collect()
}

pub fn all_king_moves(src: Square) -> Vec<Move> {
    let mut moves = Vec::with_capacity(4);
    for &(file_delta, rank_delta) in &KING_OFFSETS {
        if let Some(dest) = src.jump(file_delta, rank_delta) {
            moves.push(Move::new(src, dest, MoveKind::Quiet));
            moves.push(Move::new(src, dest, MoveKind::Capture));
        }
    }

    if src.to_board() & BitBoard::KING_START_POS != BitBoard::EMPTY {
        moves.push(Move::new(
            src,
            Square::from_row_col(src.row(), 6),
            MoveKind::Castle,
        ));
        moves.push(Move::new(
            src,
            Square::from_row_col(src.row(), 2),
            MoveKind::Castle,
        ));
    }

    moves
}
