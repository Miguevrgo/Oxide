use super::evaluation::evaluate;
use super::tt::{Bound, TTEntry, TranspositionTable};
use crate::engine::evaluation::PIECE_VALUES;
use crate::game::moves::MoveKind;
use crate::game::{board::Board, moves::Move};
use std::time::Instant;

const INF: i32 = 2 << 16;
const MATE: i32 = INF << 2;
const MAX_DEPTH: usize = 16;

pub fn find_best_move(board: &Board, depth: usize) -> Move {
    let start = Instant::now();
    let mut moves = board.generate_legal_moves();
    if moves.is_empty() {
        return Move::default();
    }

    moves.sort_unstable_by_key(|m| {
        std::cmp::Reverse({
            let mut board_new = *board;
            board_new.make_move(*m);
            evaluate(&board_new)
        })
    });

    let mut tt = TranspositionTable::new();

    let mut best_move = Move::default();
    let mut best_eval = -INF;

    for m in moves {
        let mut new_board = *board;
        new_board.make_move(m);
        let eval = -negamax(&new_board, depth - 1, -INF, INF, &mut tt);

        if eval > best_eval {
            best_move = m;
            best_eval = eval;
        }
    }

    println!("Depth: {depth} Time: {}µs", start.elapsed().as_micros());
    best_move
}

fn negamax(
    board: &Board,
    depth: usize,
    mut alpha: i32,
    beta: i32,
    tt: &mut TranspositionTable,
) -> i32 {
    let key = board.hash.0;

    if let Some(entry) = tt.get(key) {
        if entry.depth >= depth {
            match entry.bound {
                Bound::Exact => return entry.value,
                Bound::LowerBound if entry.value >= beta => return entry.value,
                Bound::UpperBound if entry.value <= alpha => return entry.value,
                _ => {}
            }
        }
    }

    if depth == 0 {
        let eval = evaluate(board);
        tt.insert(
            key,
            TTEntry {
                depth,
                value: eval,
                bound: Bound::Exact,
            },
        );
        return eval;
    }

    let mut moves = board.generate_legal_moves();
    if moves.is_empty() {
        let king_square = board.king_square(board.side);
        return if board.is_attacked_by(king_square, !board.side) {
            -MATE - depth as i32
        } else {
            0 // Draw
        };
    }

    moves.sort_by_key(|m| std::cmp::Reverse(move_score(m, board)));

    let old_alpha = alpha;
    let mut max_score = -INF;
    for m in moves {
        let mut new_board = *board;
        new_board.make_move(m);
        let score = -negamax(&new_board, depth - 1, -beta, -alpha, tt);

        max_score = std::cmp::max(score, max_score);
        alpha = std::cmp::max(alpha, score);

        if alpha >= beta {
            break; // Beta cutoff
        }
    }

    let bound = if max_score <= old_alpha {
        Bound::UpperBound
    } else if max_score >= beta {
        Bound::LowerBound
    } else {
        Bound::Exact
    };

    tt.insert(
        key,
        TTEntry {
            depth,
            value: max_score,
            bound,
        },
    );

    max_score
}

fn move_score(m: &Move, board: &Board) -> i32 {
    let mut score = 0;

    if m.get_type().is_capture() {
        let src_piece = board.piece_at(m.get_source()).unwrap();
        let dest_piece = board.piece_at(m.get_dest());

        if let Some(dest_piece) = dest_piece {
            score += 10 * PIECE_VALUES[dest_piece.index()] - PIECE_VALUES[src_piece.index()];
        } else if m.get_type() == MoveKind::EnPassant {
            score += PIECE_VALUES[0];
        }
    }

    if m.get_type().is_promotion() {
        let promo_piece = m.get_type().get_promotion(board.side);
        score += PIECE_VALUES[promo_piece.index()];
    }

    let mut new_board = *board;
    new_board.make_move(*m);
    if new_board.is_attacked_by(new_board.king_square(!board.side), board.side) {
        score += 50;
    }

    score
}
