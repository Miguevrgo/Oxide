use super::{
    bitboard::BitBoard,
    board::Board,
    castle::CastlingRights,
    constants::{CASTLE_KEYS, EP_KEYS, PIECE_KEYS, SIDE_KEY},
    piece::{Colour, Piece},
    square::Square,
};

#[derive(Debug, Eq, Clone, Copy, PartialEq, PartialOrd)]
pub struct ZHash(pub u64);

impl ZHash {
    pub const NULL: Self = Self(0);

    pub fn new(board: &Board) -> Self {
        let mut hash = Self::NULL;

        let mut occ = board.sides[Colour::White as usize] | board.sides[Colour::Black as usize];
        while occ != BitBoard::EMPTY {
            let sq = occ.pop_lsb();
            hash.hash_piece(board.piece_at(sq), sq);
        }

        if let Some(square) = board.en_passant {
            hash.hash_enpassant(square);
        }

        hash.hash_castle(board.castling_rights);

        if board.side == Colour::White {
            hash.hash_side();
        }

        hash
    }

    pub fn hash_piece(&mut self, piece: Piece, square: Square) {
        self.0 ^= PIECE_KEYS[piece as usize][square.index()];
    }

    pub fn hash_enpassant(&mut self, square: Square) {
        self.0 ^= EP_KEYS[square.index()]
    }

    pub fn swap_castle(&mut self, old_rights: CastlingRights, new_rights: CastlingRights) {
        self.0 ^= CASTLE_KEYS[old_rights.index()];
        self.0 ^= CASTLE_KEYS[new_rights.index()]
    }

    pub fn hash_castle(&mut self, castle: CastlingRights) {
        self.0 ^= CASTLE_KEYS[castle.index()]
    }

    pub fn hash_side(&mut self) {
        self.0 ^= SIDE_KEY
    }
}
