use super::evaluation::evaluate;
use crate::engine::evaluation::PIECE_VALUES;
use crate::game::moves::MoveKind;
use crate::game::{board::Board, moves::Move};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

const INF: i32 = 16384;
const MATE: i32 = 16300;

pub fn find_best_move(board: &Board, mut depth: usize) -> Move {
    let start = Instant::now();
    let mut moves = board.generate_legal_moves();
    if moves.is_empty() {
        return Move::default();
    }

    if board.occupied() <= 18 {
        depth += 1;
        if board.occupied() <= 12 {
            depth += 2;
        }
    }

    moves.sort_unstable_by_key(|m| {
        std::cmp::Reverse({
            let mut board_new = *board;
            board_new.make_move(*m);
            evaluate(&board_new)
        })
    });

    let (tx, rx) = mpsc::channel();
    let mut handles = vec![];

    for m in moves {
        let board_clone = *board;
        let tx_clone = tx.clone();

        let handle = thread::spawn(move || {
            let mut new_board = board_clone;
            new_board.make_move(m);
            let eval = -negamax(&new_board, depth - 1, -INF, INF);
            tx_clone.send((eval, m)).unwrap();
        });
        handles.push(handle);
    }

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        results.push(rx.recv().unwrap());
        handle.join().unwrap();
    }

    println!("Elapsed: {}", start.elapsed().as_micros());
    results
        .into_iter()
        .max_by_key(|&(eval, _)| eval)
        .map(|(_, mv)| mv)
        .unwrap_or(Move::default())
}

fn negamax(board: &Board, depth: usize, mut alpha: i32, beta: i32) -> i32 {
    if depth == 0 {
        return evaluate(board);
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

    let mut max_score = -INF;
    for m in moves {
        let mut new_board = *board;
        new_board.make_move(m);
        let score = -negamax(&new_board, depth - 1, -beta, -alpha);

        max_score = std::cmp::max(score, max_score);
        alpha = std::cmp::max(alpha, score);

        if alpha >= beta {
            break; // Beta cutoff
        }
    }

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
