use crate::engine::tables::{history_bonus, Bound, SearchData};
use crate::game::moves::MoveKind;
use crate::game::{board::Board, moves::Move};

pub const INF: i32 = 2 << 16;
pub const MATE: i32 = INF >> 2;
pub const DRAW: i32 = 0;
pub const MAX_DEPTH: u8 = 64;

// Move Scores
const TT_SCORE: i32 = 10_000_000;
const PROM_SCORE: i32 = 80_000;
const CAP_SCORE: i32 = 90_000;
const KILL_SCORE: i32 = 70_000;

// Search Parameters
const ASPIRATION_DELTA: i32 = 50;
const ASPIRATION_DELTA_LIMIT: i32 = 400;

const NMP_MIN_DEPTH: u8 = 3;
const NMP_BASE_REDUCTION: u8 = 4;
const NMP_DIVISOR: u8 = 4;

const RFP_DEPTH: u8 = 8;
const RFP_IMPROVING: i32 = 50;
const RFP_MARGIN: i32 = 90;
const LMR_DIV: f64 = 2.55;
const LMR_BASE: f64 = 0.55;

const RAZOR_DEPTH: u8 = 3;
const RAZOR_MARGIN: i32 = 420;

pub const HISTORY_MAX_BONUS: i16 = 1500;
pub const HISTORY_FACTOR: i16 = 355;
pub const HISTORY_OFFSET: i16 = 345;
pub const MAX_CAP_HISTORY: i32 = 16384;
pub const MAX_HISTORY: i32 = 8192;

pub fn find_best_move(board: &Board, max_depth: u8, data: &mut SearchData) {
    data.start_search();

    while data.depth <= max_depth && !data.stop {
        data.eval = if data.depth < 5 {
            negamax(board, data.depth, -INF, INF, data)
        } else {
            aspiration_window(board, data.depth, data.eval, data)
        };

        if data.stop {
            break;
        } else if data.timing.elapsed().as_millis() * 5 / 4 > data.time_tp
            || data.eval.abs() >= MATE - i32::from(MAX_DEPTH)
        {
            data.stop = true;
        }

        println!("{data}");
        data.depth += 1;
    }
}

fn aspiration_window(board: &Board, max_depth: u8, estimate: i32, data: &mut SearchData) -> i32 {
    let mut delta = ASPIRATION_DELTA;
    let mut alpha = estimate - delta;
    let mut beta = estimate + delta;
    let mut depth = max_depth;

    loop {
        let score = negamax(board, depth, alpha, beta, data);
        if data.stop {
            return 0;
        }

        if score <= alpha {
            beta = (alpha + beta) / 2;
            alpha = (-INF).max(alpha - delta);
            depth = max_depth;
        } else if score >= beta {
            beta = INF.min(beta + delta);
            if depth > 1 {
                depth -= 1;
            }
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

fn quiescence(board: &Board, mut alpha: i32, beta: i32, data: &mut SearchData) -> i32 {
    let key = board.hash.0;
    if let Some(entry) = data.tt.probe(key) {
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
        return best_eval;
    }

    alpha = alpha.max(best_eval);

    let mut moves = board.generate_legal_moves::<false>();
    let mut scores = [0; 252];
    moves.as_slice().iter().enumerate().for_each(|(i, m)| {
        scores[i] = mvv_lva(*m, board);
    });

    let mut best_move = Move::NULL;
    let mut bound = Bound::Upper;

    data.ply += 1;

    while let Some((m, _)) = moves.pick(&mut scores) {
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

    data.tt.insert(key, bound, best_move, best_eval, 0, false);

    best_eval
}

fn negamax(board: &Board, mut depth: u8, mut alpha: i32, beta: i32, data: &mut SearchData) -> i32 {
    if data.stop || (data.nodes & 4095 == 0 && !data.continue_search()) {
        data.stop = true;
        return 0;
    }

    let in_check = board.in_check();
    let key = board.hash.0;
    data.ply_data[data.ply].pv.clear();

    if data.ply > 0 && depth < MAX_DEPTH {
        if board.is_draw() || data.is_repetition(board, key, false) {
            return DRAW;
        }
        // Check Extensions
        depth += u8::from(in_check);
    }

    if depth == 0 {
        return quiescence(board, alpha, beta, data);
    }

    let pv_node = beta > alpha + 1;
    let mut tt_move = None;
    if let Some(entry) = data.tt.probe(key) {
        tt_move = Some(entry.best_move);
        if entry.depth() >= depth && !pv_node {
            match entry.bound() {
                Bound::Exact => return entry.value,
                Bound::Lower if entry.value >= beta => return entry.value,
                Bound::Upper if entry.value <= alpha => return entry.value,
                _ => {}
            }
        }
    }

    let can_prune = !pv_node && !in_check;
    if can_prune {
        // Reverse Futility pruning
        let static_eval = board.evaluate(&mut data.cache);
        data.ply_data[data.ply].eval = static_eval;
        let improving = data.ply >= 2 && static_eval > data.ply_data[data.ply - 2].eval;
        let rfp_margin = RFP_MARGIN * depth as i32 - RFP_IMPROVING * improving as i32;

        if depth <= RFP_DEPTH && static_eval - rfp_margin >= beta {
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
        if depth >= NMP_MIN_DEPTH && !board.is_king_pawn() {
            let mut null_board = *board;
            null_board.make_null_move();
            let r = (NMP_BASE_REDUCTION + depth / NMP_DIVISOR).min(depth);
            let null_score = -negamax(&null_board, depth - r, -beta, -beta + 1, data);
            if null_score >= beta {
                return null_score;
            }
        }
    }

    // Internal Iterative Reduction
    if depth >= 4 && tt_move.is_none() {
        depth -= 1;
    }

    let mut moves = board.generate_legal_moves::<true>();
    if moves.is_empty() {
        return i32::from(in_check) * (data.ply as i32 - MATE);
    }
    let mut scores = [0; 252];

    moves.as_slice().iter().enumerate().for_each(|(i, m)| {
        scores[i] = move_score(m, board, tt_move, data.ply, data);
    });

    let old_alpha = alpha;
    let mut best_move = Move::NULL;
    let mut best_score = -INF;
    let mut move_idx = 0;
    let lmr_ready = depth > 1 && !in_check;
    let lmr_depth = (depth as f64).ln() / (LMR_DIV);
    let can_prune = !pv_node && !in_check;
    let mut quiets_tried = Vec::with_capacity(16);
    let mut caps_tried = Vec::with_capacity(16);
    data.push(key);

    while let Some((m, ms)) = moves.pick(&mut scores) {
        if can_prune && best_score.abs() < MATE {
            // History pruning
            if depth <= 2 && ms < -4000 {
                break;
            }
        }

        let mut new_board = *board;
        new_board.make_move(m);
        move_idx += 1;
        data.nodes += 1;

        let new_in_check = new_board.in_check();
        let mut reduction = 0;

        // Late Move Reduction
        if lmr_ready && ms < KILL_SCORE {
            reduction = (LMR_BASE + lmr_depth * (move_idx as f64).ln()) as i16;
            reduction -= i16::from(pv_node);
            reduction -= i16::from(new_in_check);
            reduction = reduction.clamp(0, depth as i16 - 1);
        }

        let score = if move_idx == 1 {
            -negamax(&new_board, depth - 1, -beta, -alpha, data)
        } else {
            let mut zw_search = -negamax(
                &new_board,
                depth - 1 - reduction as u8,
                -alpha - 1,
                -alpha,
                data,
            );

            if zw_search > alpha && (pv_node || reduction > 0) {
                zw_search = -negamax(&new_board, depth - 1, -beta, -alpha, data);
            }
            zw_search
        };

        if score > best_score {
            best_score = score;
            best_move = m;
            if pv_node {
                let pre_line = data.ply_data[data.ply].pv;
                let full_line = &mut data.ply_data[data.ply - 1].pv;
                full_line.update_pv_line(m, &pre_line);
            }
        }

        alpha = alpha.max(score);

        if alpha >= beta {
            let history_bonus = history_bonus(depth);
            if !m.get_type().is_capture() {
                let killers = &mut data.ply_data[data.ply].killers;
                if killers[0] != m {
                    killers[1] = killers[0];
                    killers[0] = m;
                }

                data.history.update(
                    board.side,
                    m.get_source().index(),
                    m.get_dest().index(),
                    history_bonus,
                    &quiets_tried,
                );
            }
            data.cap_history
                .update(board, m, history_bonus, &caps_tried);

            break;
        }

        if !m.get_type().is_capture() {
            quiets_tried.push(m);
        } else {
            caps_tried.push(m);
        }
    }

    data.pop();

    if data.stop {
        return 0;
    }

    let bound = if best_score <= old_alpha {
        Bound::Upper
    } else if best_score >= beta {
        Bound::Lower
    } else {
        Bound::Exact
    };

    data.tt
        .insert(key, bound, best_move, best_score, depth, pv_node);

    if data.ply == 0 {
        data.best_move = best_move
    }

    best_score
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
    if Some(*m) == tt_move {
        return TT_SCORE;
    }

    let kind = m.get_type();

    if kind == MoveKind::QueenPromotion {
        return PROM_SCORE;
    }

    if kind.is_capture() {
        let see = board.see(*m, 0);
        return CAP_SCORE * see as i32
            + mvv_lva(*m, board)
            + data.cap_history.score[board.piece_at(m.get_source()) as usize][m.get_dest().index()]
                [board.capture_piece(*m).index()] as i32;
    }

    let killers = data.ply_data[ply].killers;
    if *m == killers[0] {
        return KILL_SCORE + 1;
    } else if *m == killers[1] {
        return KILL_SCORE;
    }

    data.history.score[board.side as usize][m.get_source().index()][m.get_dest().index()] as i32
}
