use crate::game::{
    bitboard::BitBoard,
    board::Board,
    moves::Move,
    piece::{Colour, Piece},
};

#[derive(Clone, Debug)]
pub struct Position {
    pub board: Board,
    pub history: Vec<Board>,
}
