use super::tt::{Bound, TTEntry, TranspositionTable};
use crate::engine::network::EvalTable;
use crate::game::constants::PIECE_VALUES;
use crate::game::moves::MoveKind;
use crate::game::piece::Piece;
use crate::game::{board::Board, moves::Move};
use std::time::Instant;

const INF: i32 = 2 << 16;
const MATE: i32 = INF >> 2;
const DRAW: i32 = 0;
const MAX_DEPTH: usize = 16;
static NODE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn find_best_move(board: &Board, max_depth: usize) -> Move {
    let mut best_move = Move::default();
    let mut tt = TranspositionTable::new();
    let mut cache = EvalTable::default();
    let mut best_eval = -INF;
    let start = Instant::now();
    let final_depth = std::cmp::min(max_depth, MAX_DEPTH);

    NODE_COUNT.store(0, std::sync::atomic::Ordering::Relaxed);
    for depth in 1..=final_depth {
        let mut moves = board.generate_legal_moves::<true>();

        if moves.is_empty() {
            return Move::default();
        }

        if let Some(entry) = tt.get(board.hash.0) {
            if let Some(i) = moves.iter().position(|&m| m == entry.best_move) {
                moves.swap(0, i);
            }
        }

        moves[1..].sort_unstable_by_key(|m| {
            std::cmp::Reverse({
                let mut b = *board;
                b.make_move(*m);
                b.evaluate(&mut cache)
            })
        });

        let mut local_best_eval = -INF;
        let mut local_best_move = Move::default();

        for &m in &moves {
            let mut new_board = *board;
            new_board.make_move(m);

            let delta = 50;
            let mut alpha = best_eval - delta;
            let mut beta = best_eval + delta;
            let mut eval;

            loop {
                eval = -negamax(&new_board, depth - 1, -beta, -alpha, &mut tt, &mut cache);
                if eval <= alpha {
                    alpha -= delta;
                } else if eval >= beta {
                    beta += delta;
                } else {
                    break;
                }
            }

            if eval > local_best_eval {
                local_best_eval = eval;
                local_best_move = m;
            }
        }

        best_eval = local_best_eval;
        best_move = local_best_move;

        let time = start.elapsed().as_millis();
        let nodes = NODE_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let nps = if time > 0 {
            (1000 * nodes as u128 / time) as u64
        } else {
            0
        };

        if best_eval.abs() > MATE - MAX_DEPTH as i32 {
            println!(
                "info depth {depth} score mate {best_eval} time {time} nodes {nodes} nps {nps}"
            );
            break;
        } else {
            println!("info depth {depth} score cp {best_eval} time {time} nodes {nodes} nps {nps}");
        }
    }

    best_move
}

fn negamax(
    board: &Board,
    depth: usize,
    mut alpha: i32,
    beta: i32,
    tt: &mut TranspositionTable,
    cache: &mut EvalTable,
) -> i32 {
    NODE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let key = board.hash.0;
    let tt_move = tt.get(key).map(|entry| entry.best_move);

    if let Some(entry) = tt.get(key) {
        if entry.depth >= depth {
            match entry.bound {
                Bound::Exact => return entry.value,
                Bound::Lower if entry.value >= beta => return entry.value,
                Bound::Upper if entry.value <= alpha => return entry.value,
                _ => {}
            }
        }
    }

    if depth == 0 {
        return quiesce(board, alpha, beta, cache);
    } else if board.is_draw() {
        return DRAW; //TODO: Add Repetition
    }

    // Null Move Pruning
    if depth >= 3 && !board.in_check() {
        let mut null_board = *board;
        null_board.make_null_move();
        let r = 2;
        let null_score = -negamax(&null_board, depth - r - 1, -beta, -beta + 1, tt, cache);
        if null_score >= beta {
            return null_score;
        }
    }

    let mut moves = board.generate_legal_moves::<true>();
    if moves.is_empty() {
        let king_square = board.king_square(board.side);
        return if board.is_attacked_by(king_square, !board.side) {
            -MATE - depth as i32
        } else {
            0 // Draw
        };
    }

    moves.sort_unstable_by_key(|m| std::cmp::Reverse(move_score(m, board, tt_move)));
    if let Some(m) = tt_move {
        if let Some(i) = moves.iter().position(|&x| x == m) {
            moves.swap(0, i);
        }
    }

    let mut best_move: Option<Move> = None;
    let old_alpha = alpha;
    let mut max_score = -INF;
    for m in moves {
        let mut new_board = *board;
        new_board.make_move(m);
        let score = -negamax(&new_board, depth - 1, -beta, -alpha, tt, cache);

        if score > max_score {
            max_score = score;
            best_move = Some(m);
        }
        alpha = std::cmp::max(alpha, score);

        if alpha >= beta {
            break; // Beta cutoff
        }
    }

    let bound = if max_score <= old_alpha {
        Bound::Upper
    } else if max_score >= beta {
        Bound::Lower
    } else {
        Bound::Exact
    };

    tt.insert(
        key,
        TTEntry {
            depth,
            value: max_score,
            bound,
            best_move: best_move.unwrap_or_default(),
        },
    );

    max_score
}

fn quiesce(board: &Board, mut alpha: i32, beta: i32, cache: &mut EvalTable) -> i32 {
    NODE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let stand_pat = board.evaluate(cache);
    if stand_pat >= beta {
        return beta;
    }

    alpha = alpha.max(stand_pat);

    let moves = board.generate_legal_moves::<false>();

    for m in moves {
        let mut new_board = *board;
        new_board.make_move(m);
        let score = -quiesce(&new_board, -beta, -alpha, cache);
        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }

    alpha
}

fn move_score(m: &Move, board: &Board, tt_move: Option<Move>) -> i32 {
    if Some(*m) == tt_move {
        return 10_000_000;
    }

    match m.get_type() {
        MoveKind::QueenPromotion => {
            let promo = m.get_type().get_promotion(board.side);
            9_000_000 + PIECE_VALUES[promo.index()]
        }
        MoveKind::Capture | MoveKind::EnPassant => {
            let src_piece = board.piece_at(m.get_source()).unwrap();
            let dst_piece = if m.get_type() == MoveKind::EnPassant {
                Some(Piece::WP)
            } else {
                board.piece_at(m.get_dest())
            };

            if let Some(victim) = dst_piece {
                8_000_000 + 10 * PIECE_VALUES[victim.index()] - PIECE_VALUES[src_piece.index()]
            } else {
                0
            }
        }
        _ => 0, // quiets
    }
}
