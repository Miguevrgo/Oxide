use crate::game::{
    bitboard::BitBoard,
    board::Board,
    piece::{Colour, Piece},
};

use super::network::{Accumulator, Network};

// Improved material values (centipawns)
pub const PIECE_VALUES: [i32; 6] = [
    100,   // Pawn
    320,   // Knight
    330,   // Bishop
    500,   // Rook
    900,   // Queen
    20000, // King (not actually used in evaluation)
];

pub fn evaluate(board: &Board) -> i32 {
    let white_king_sq = board.king_square(Colour::White).index();
    let black_king_sq = board.king_square(Colour::Black).index();

    let mut white_acc = Accumulator::default();
    let mut black_acc = Accumulator::default();

    fill_features(
        board,
        &mut white_acc,
        &mut black_acc,
        white_king_sq,
        black_king_sq,
    );

    let eval = if board.side == Colour::White {
        Network::out(&white_acc, &black_acc)
    } else {
        Network::out(&black_acc, &white_acc)
    };

    scale(board, eval)
}

fn fill_features(
    board: &Board,
    white_acc: &mut Accumulator,
    black_acc: &mut Accumulator,
    white_king_sq: usize,
    black_king_sq: usize,
) {
    let wflip = if white_king_sq % 8 > 3 { 7 } else { 0 };
    let bflip = if black_king_sq % 8 > 3 { 7 } else { 0 } ^ 56;

    let occ = board.sides[Colour::White as usize] | board.sides[Colour::Black as usize];
    let mut occupied = occ;

    while occupied != BitBoard::EMPTY {
        let sq = occupied.lsb();
        if let Some(piece) = board.piece_at(sq) {
            let side = piece.colour() as usize;
            let pc = piece.index();

            let wbase = Network::get_base_index::<0>(side, pc, white_king_sq) as u16;
            let bbase = Network::get_base_index::<1>(side, pc, black_king_sq) as u16;
            let wfeat = wbase + (sq.index() as u16 ^ wflip);
            let bfeat = bbase + (sq.index() as u16 ^ bflip);

            white_acc.update_multi(&[wfeat]);
            black_acc.update_multi(&[bfeat]);
        }
        occupied = occupied.pop_bit(sq);
    }
}

fn scale(board: &Board, eval: i32) -> i32 {
    let mut mat = (board.pieces[Piece::WN.index()].count_bits() as i32
        * PIECE_VALUES[Piece::WN.index()])
        + (board.pieces[Piece::WB.index()].count_bits() as i32 * PIECE_VALUES[Piece::WB.index()])
        + (board.pieces[Piece::WR.index()].count_bits() as i32 * PIECE_VALUES[Piece::WR.index()])
        + (board.pieces[Piece::WQ.index()].count_bits() as i32 * PIECE_VALUES[Piece::WQ.index()]);

    mat = 700 + mat / 32;
    eval * mat / 1024
}
