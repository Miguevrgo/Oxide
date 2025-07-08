use crate::engine::tables::{Bound, SearchData, TTEntry};
use crate::game::moves::MoveKind;
use crate::game::{board::Board, moves::Move};
use std::time::Instant;

pub const INF: i32 = 2 << 16;
const MATE: i32 = INF >> 2;
const DRAW: i32 = 0;
const MAX_DEPTH: u8 = 32;

// Search Parameters
pub const ASPIRATION_DELTA: i32 = 50;
pub const ASPIRATION_DELTA_LIMIT: i32 = 200;

pub const NMP_MIN_DEPTH: u8 = 3;
pub const NMP_BASE_REDUCTION: u8 = 4;
pub const NMP_DIVISOR: u8 = 4;

pub const RFP_DEPTH: u8 = 6;
pub const LMR_DEPTH: u8 = 2;
pub const LMR_THRESHOLD: usize = 2;

pub const RAZOR_DEPTH: u8 = 3;
pub const RAZOR_MARGIN: i32 = 420;

pub fn find_best_move(
    board: &Board,
    max_depth: Option<u8>,
    time_play: u128,
    data: &mut SearchData,
) {
    let mut depth = 1;
    let mut stop = false;
    let final_depth = max_depth.unwrap_or(MAX_DEPTH);
    data.best_move = Move::NULL;
    data.push(board.hash.0);
    data.nodes = 0;
    data.timing = Instant::now();

    let mut moves = board.generate_legal_moves::<true>();
    moves
        .as_mut_slice()
        .sort_unstable_by_key(|m| std::cmp::Reverse(move_score(m, board, None, data.ply, data)));

    while depth <= final_depth && !stop {
        if let Some(entry) = data.tt.get(board.hash.0) {
            if let Some(i) = moves
                .as_mut_slice()
                .iter()
                .position(|&m| m == entry.best_move)
            {
                moves.as_mut_slice().swap(0, i);
            }
        }

        let mut local_best_eval = -INF;
        let mut local_best_move = Move::default();

        for m in &moves {
            let mut new_board = *board;
            new_board.make_move(m);
            data.push(new_board.hash.0);

            let eval = if depth < 5 {
                -negamax(&new_board, depth - 1, -INF, INF, data)
            } else {
                aspiration_window(&new_board, depth - 1, data.eval, data)
            };

            data.pop();

            if eval > local_best_eval {
                local_best_eval = eval;
                local_best_move = m;
            }
        }

        data.eval = local_best_eval;
        data.best_move = local_best_move;

        let time = data.timing.elapsed().as_millis();
        if time * 2 > time_play && depth >= 4 && final_depth == MAX_DEPTH {
            stop = true;
        }
        let nodes = data.nodes;
        let nps = if time > 0 {
            (1000 * nodes as u128 / time) as u64
        } else {
            0
        };

        if data.eval.abs() >= MATE - i32::from(MAX_DEPTH) {
            let mate_in = (MATE - data.eval.abs()) / 2;
            let sign = if data.eval < 0 { "-" } else { "" };
            println!(
                "info depth {depth} score mate {sign}{mate_in} time {time} nodes {nodes} nps {nps}"
            );
            break;
        } else {
            println!(
                "info depth {depth} score cp {} time {time} nodes {nodes} nps {nps}",
                data.eval
            );
        }

        depth += 1;
    }
}

fn aspiration_window(board: &Board, max_depth: u8, estimate: i32, data: &mut SearchData) -> i32 {
    let mut delta = ASPIRATION_DELTA;
    let mut alpha = estimate - delta;
    let mut beta = estimate + delta;
    let mut depth = max_depth;

    loop {
        let score = -negamax(board, depth, -beta, -alpha, data);

        if score <= alpha {
            beta = (alpha + beta) / 2;
            alpha = (-INF).max(alpha - delta);
            depth = max_depth;
        } else if score >= beta {
            beta = INF.min(beta + delta);
            depth -= 1;
        } else {
            return score;
        }

        delta += delta / 2;
        if delta > ASPIRATION_DELTA_LIMIT {
            alpha = -INF;
            beta = INF;
        }
    }
}

fn negamax(board: &Board, mut depth: u8, mut alpha: i32, beta: i32, data: &mut SearchData) -> i32 {
    let in_check = board.in_check();
    let key = board.hash.0;

    if data.ply > 0 {
        if board.is_draw() || data.is_repetition(key, false) {
            return DRAW;
        }
        // Check Extensions
        depth += u8::from(in_check);
    }

    if depth == 0 {
        return quiescence(board, alpha, beta, data);
    } else if board.is_draw() {
        return DRAW;
    }

    let pv_node = beta > alpha + 1;
    let tt_move = data.tt.get(key).map(|entry| entry.best_move);
    if let Some(entry) = data.tt.get(key) {
        if entry.depth() >= depth && !pv_node {
            match entry.bound() {
                Bound::Exact => return entry.value,
                Bound::Lower if entry.value >= beta => return entry.value,
                Bound::Upper if entry.value <= alpha => return entry.value,
                _ => {}
            }
        }
    }

    if !in_check && !board.is_king_pawn() {
        // Reverse Futility pruning
        let static_eval = board.evaluate(&mut data.cache);
        data.ply_data[data.ply].eval = static_eval;
        let improving = data.ply >= 2 && static_eval > data.ply_data[data.ply - 2].eval;
        let rfp_margin = 100 * depth as i32 - if improving { 50 } else { 0 };

        if depth <= RFP_DEPTH && !pv_node && beta < MATE && static_eval - rfp_margin >= beta {
            return static_eval;
        }

        // Razoring
        if depth < RAZOR_DEPTH && static_eval + RAZOR_MARGIN * (depth as i32) < alpha {
            let qeval = quiescence(board, alpha, beta, data);
            if qeval < alpha {
                return qeval;
            }
        }

        // Null Move Pruning
        if depth >= NMP_MIN_DEPTH {
            let mut null_board = *board;
            null_board.make_null_move();
            let r = (NMP_BASE_REDUCTION + depth / NMP_DIVISOR).min(depth);
            let null_score = -negamax(&null_board, depth - r, -beta, -beta + 1, data);
            if null_score >= beta {
                return null_score;
            }
        }
    }

    let mut moves = board.generate_legal_moves::<true>();
    if moves.is_empty() {
        return i32::from(in_check) * (data.ply as i32 - MATE);
    }

    moves
        .as_mut_slice()
        .sort_unstable_by_key(|m| std::cmp::Reverse(move_score(m, board, tt_move, data.ply, data)));

    let mut best_move: Option<Move> = None;
    let old_alpha = alpha;
    let mut max_score = -INF;

    for (i, m) in moves.as_mut_slice().iter().enumerate() {
        let mut new_board = *board;
        new_board.make_move(*m);
        data.push(new_board.hash.0);
        data.nodes += 1;

        let mut score;

        // Principal Variation Search
        if i == 0 {
            score = -negamax(&new_board, depth - 1, -beta, -alpha, data);
        } else {
            // Late Move Reductions // TODO ln
            if depth >= LMR_DEPTH
                && i >= LMR_THRESHOLD
                && !m.get_type().is_capture()
                && !m.get_type().is_promotion()
            {
                let reduction = (depth as i32 / 3).min(2) as u8;
                score = -negamax(&new_board, depth - 1 - reduction, -alpha - 1, -alpha, data);

                if score > alpha {
                    score = -negamax(&new_board, depth - 1, -alpha - 1, -alpha, data);
                    if score > alpha {
                        score = -negamax(&new_board, depth - 1, -beta, -alpha, data);
                    }
                }
            } else {
                score = -negamax(&new_board, depth - 1, -alpha - 1, -alpha, data);
                if score > alpha {
                    score = -negamax(&new_board, depth - 1, -beta, -alpha, data);
                }
            }
        }

        if score > max_score {
            max_score = score;
            best_move = Some(*m);
        }

        alpha = alpha.max(score);
        data.pop();

        if alpha >= beta {
            if !m.get_type().is_capture() {
                let killers = &mut data.ply_data[data.ply].killers;
                if killers[0] != *m {
                    killers[1] = killers[0];
                    killers[0] = *m;
                }
            }
            break;
        }
    }

    let bound = if max_score <= old_alpha {
        Bound::Upper
    } else if max_score >= beta {
        Bound::Lower
    } else {
        Bound::Exact
    };

    data.tt.insert(
        key,
        TTEntry {
            value: max_score,
            best_move: best_move.unwrap_or_default(),
            flags: TTEntry::make_flags(depth, bound),
        },
    );

    max_score
}

fn quiescence(board: &Board, mut alpha: i32, beta: i32, data: &mut SearchData) -> i32 {
    let key = board.hash.0;
    if let Some(entry) = data.tt.get(key) {
        let tt_score = entry.value;
        match entry.bound() {
            Bound::Exact => return tt_score,
            Bound::Lower if tt_score >= beta => return tt_score,
            Bound::Upper if tt_score <= alpha => return tt_score,
            _ => {}
        }
    }

    let mut best_eval = board.evaluate(&mut data.cache);
    if best_eval >= beta {
        data.tt.insert(
            key,
            TTEntry {
                value: best_eval,
                best_move: Move::default(),
                flags: TTEntry::make_flags(0, Bound::Lower),
            },
        );
        return best_eval;
    }

    alpha = alpha.max(best_eval);

    let mut moves = board.generate_legal_moves::<false>();
    moves
        .as_mut_slice()
        .sort_unstable_by_key(|m| std::cmp::Reverse(move_score(m, board, None, data.ply, data)));

    let mut best_move = Move::default();
    let mut bound = Bound::Upper;

    data.ply += 1;
    for m in moves {
        let mut new_board = *board;
        new_board.make_move(m);
        data.nodes += 1;

        let score = -quiescence(&new_board, -beta, -alpha, data);

        if score > best_eval {
            best_eval = score;
            best_move = m;
            alpha = alpha.max(score);
        }

        if score >= beta {
            bound = Bound::Lower;
            break;
        }
    }
    data.ply -= 1;

    if best_eval > alpha {
        bound = Bound::Exact;
    }

    data.tt.insert(
        key,
        TTEntry {
            value: best_eval,
            best_move,
            flags: TTEntry::make_flags(0, bound),
        },
    );

    best_eval
}

#[inline]
fn mvv_lva(m: Move, board: &Board) -> i32 {
    let victim = board.piece_at(m.get_dest()).index() as i32;
    let attacker = board.piece_at(m.get_source()).index() as i32;
    8 * victim - attacker
}

pub fn move_score(
    m: &Move,
    board: &Board,
    tt_move: Option<Move>,
    ply: usize,
    data: &SearchData,
) -> i32 {
    // 1. TT move
    if Some(*m) == tt_move {
        return 10_000;
    }

    // 2. Promotion
    if m.get_type() == MoveKind::QueenPromotion {
        return 9_000;
    }

    // 3. Capture ~ en passant

    if m.get_type().is_capture() {
        return 8000 + mvv_lva(*m, board);
    }

    // 4. Killer moves
    let killers = data.ply_data[ply].killers;
    if *m == killers[0] {
        return 7_000;
    } else if *m == killers[1] {
        return 6_900;
    }

    0
}
