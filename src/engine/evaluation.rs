use crate::game::{
    bitboard::BitBoard,
    board::Board,
    piece::{Colour, Piece},
};

use super::network::*;

pub fn evaluate(board: &Board, cache: &mut EvalTable) -> i32 {
    let white_king_sq = board.king_square(Colour::White).index();
    let black_king_sq = board.king_square(Colour::Black).index();

    let wbucket = Network::get_bucket::<0>(white_king_sq);
    let bbucket = Network::get_bucket::<1>(black_king_sq);

    let entry = &mut cache.table[wbucket][bbucket];

    let mut addf = [[0; 32]; 2];
    let mut subf = [[0; 32]; 2];
    let (adds, subs) = fill_diff(
        board,
        &entry.bbs,
        &mut addf,
        &mut subf,
        white_king_sq,
        black_king_sq,
    );

    entry.white.update_multi(&addf[0][..adds], &subf[0][..subs]);
    entry.black.update_multi(&addf[1][..adds], &subf[1][..subs]);

    entry.bbs = [
        board.sides[Colour::White as usize].0,
        board.sides[Colour::Black as usize].0,
        board.pieces[Piece::WP.index()].0,
        board.pieces[Piece::WN.index()].0,
        board.pieces[Piece::WB.index()].0,
        board.pieces[Piece::WR.index()].0,
        board.pieces[Piece::WQ.index()].0,
        board.pieces[Piece::WK.index()].0,
    ];

    let eval = if board.side == Colour::White {
        Network::out(&entry.white, &entry.black)
    } else {
        Network::out(&entry.black, &entry.white)
    };

    board.scale(eval)
}

fn fill_diff(
    board: &Board,
    bbs: &[u64; 8],
    add_feats: &mut [[u16; 32]; 2],
    sub_feats: &mut [[u16; 32]; 2],
    white_king_sq: usize,
    black_king_sq: usize,
) -> (usize, usize) {
    let mut adds = 0;
    let mut subs = 0;

    let wflip = if white_king_sq % 8 > 3 { 7 } else { 0 };
    let bflip = if black_king_sq % 8 > 3 { 7 } else { 0 } ^ 56;

    for side in [Colour::White as usize, Colour::Black as usize] {
        let old_boys = bbs[side];
        let new_boys = board.sides[side].0;

        for (piece, &old_bb) in bbs[2..8].iter().enumerate() {
            let old_bb = old_bb & old_boys;
            let new_bb = board.pieces[piece].0 & new_boys;

            let wbase = Network::get_base_index::<0>(side, piece, white_king_sq) as u16;
            let bbase = Network::get_base_index::<1>(side, piece, black_king_sq) as u16;

            let mut add_diff = BitBoard(new_bb & !old_bb);
            while add_diff != BitBoard::EMPTY {
                let sq = add_diff.lsb();
                let sq_idx = sq.index() as u16;
                add_feats[0][adds] = wbase + (sq_idx ^ wflip);
                add_feats[1][adds] = bbase + (sq_idx ^ bflip);
                adds += 1;
                add_diff = add_diff.pop_bit(sq);
            }

            let mut sub_diff = BitBoard(old_bb & !new_bb);
            while sub_diff != BitBoard::EMPTY {
                let sq = sub_diff.lsb();
                let sq_idx = sq.index() as u16;
                sub_feats[0][subs] = wbase + (sq_idx ^ wflip);
                sub_feats[1][subs] = bbase + (sq_idx ^ bflip);
                subs += 1;
                sub_diff = sub_diff.pop_bit(sq);
            }
        }
    }

    (adds, subs)
}
