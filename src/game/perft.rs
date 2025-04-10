use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Instant;

use crate::game::board::Board;
use crate::game::moves::Move;

pub const BULK: bool = true;
pub const NO_BULK: bool = false;
pub const NUM_THREADS: usize = 16;

pub fn perft_with_moves(board: &mut Board, depth: usize) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = board.generate_legal_moves();
    let mut total = 0;
    if depth == 1 {
        for m in &moves {
            let mut new_board = *board;
            new_board.make_move(*m);
            let count = perft_with_moves(&mut new_board, depth - 1);
            println!("{}: {}", m, count);
            total += count;
        }
    } else {
        for m in &moves {
            let mut new_board = *board;
            new_board.make_move(*m);
            total += perft_with_moves(&mut new_board, depth - 1);
        }
    }
    total
}

impl Board {
    fn perft_driver<const BULK_COUNT: bool>(
        &mut self,
        depth: usize,
        level_counts: &mut Vec<u64>,
    ) -> u64 {
        if depth == 0 {
            return 1;
        }

        let moves = self.generate_legal_moves();
        let current_level = level_counts.len() - depth;
        if current_level < level_counts.len() {
            level_counts[current_level] += moves.len() as u64;
        }

        if BULK_COUNT && depth == 1 {
            return moves.len() as u64;
        }

        let mut nodes = 0;
        for m in moves {
            let mut new_board = *self;
            new_board.make_move(m);
            nodes += new_board.perft_driver::<BULK_COUNT>(depth - 1, level_counts);
        }
        nodes
    }

    pub fn perft<const BULK_COUNT: bool>(&mut self, depth: usize) -> u64 {
        if depth == 0 {
            return 1;
        }

        let moves = self.generate_legal_moves();
        if moves.is_empty() {
            return 0;
        }

        let moves_per_thread = moves.len().div_ceil(NUM_THREADS);

        let (tx, rx): (
            Sender<(Move, u64, Vec<u64>)>,
            Receiver<(Move, u64, Vec<u64>)>,
        ) = channel();
        let mut handles = Vec::new();
        let mut total_level_counts = vec![0u64; depth];

        let start = Instant::now();

        for thread_id in 0..NUM_THREADS {
            let start_idx = thread_id * moves_per_thread;
            let end_idx = usize::min(start_idx + moves_per_thread, moves.len());
            if start_idx >= moves.len() {
                break;
            }

            let moves_chunk = moves[start_idx..end_idx].to_vec();
            let board_clone = *self;
            let tx_clone = tx.clone();

            let handle = thread::spawn(move || {
                let mut thread_level_counts = vec![0u64; depth];
                for m in moves_chunk {
                    let mut new_board = board_clone;
                    new_board.make_move(m);
                    let nodes =
                        new_board.perft_driver::<BULK_COUNT>(depth - 1, &mut thread_level_counts);
                    tx_clone
                        .send((m, nodes, thread_level_counts.clone()))
                        .expect("Failed to send result");
                    thread_level_counts = vec![0u64; depth];
                }
            });
            handles.push(handle);
        }

        drop(tx);

        let mut results = Vec::new();
        while let Ok((m, nodes, thread_counts)) = rx.recv() {
            results.push((m, nodes));
            for (i, &count) in thread_counts.iter().enumerate() {
                total_level_counts[i] += count;
            }
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        total_level_counts[0] = moves.len() as u64;

        let total_nodes: u64 = results.iter().map(|(_, nodes)| nodes).sum();
        let total_duration = start.elapsed();

        #[cfg(debug_assertions)]
        {
            for (m, nodes) in &results {
                println!("{}{}: {}", m.get_source(), m.get_dest(), nodes);
            }
        }

        println!("\nMoves per level:");
        for (i, &count) in total_level_counts.iter().enumerate() {
            println!("Depth {}: {} moves", i, count);
        }

        let nodes_per_sec = if total_duration.as_micros() > 0 {
            (total_nodes as f64 / total_duration.as_micros() as f64) * 1_000_000.0
        } else {
            0.0
        };
        println!(
            "\nTotal: {} nodes in {:.3}s - {:.2} Mnps",
            total_nodes,
            total_duration.as_secs_f64(),
            nodes_per_sec / 1_000_000.0
        );

        total_nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perft_suite() {
        const PERFT_SUITE: [(&str, &str, u64, usize); 16] = [
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                "Startpos",
                119_060_324,
                6,
            ),
            (
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                "Kiwipete",
                193_690_690,
                5,
            ),
            (
                "8/8/4k3/8/2p5/8/B2P2K1/8 w - - 0 1",
                "Illegal ep move #1",
                1_015_133,
                6,
            ),
            (
                "3k4/3p4/8/K1P4r/8/8/8/8 b - - 0 1",
                "Illegal ep move #2",
                1_134_888,
                6,
            ),
            (
                "8/8/1k6/2b5/2pP4/8/5K2/8 b - d3 0 1",
                "Ep capture checks opponent",
                1_440_467,
                6,
            ),
            (
                "5k2/8/8/8/8/8/8/4K2R w K - 0 1",
                "Short castling gives check",
                661_072,
                6,
            ),
            (
                "3k4/8/8/8/8/8/8/R3K3 w Q - 0 1",
                "Long castling gives check",
                803_711,
                6,
            ),
            (
                "r3k2r/1b4bq/8/8/8/8/7B/R3K2R w KQkq - 0 1",
                "Castle rights",
                1_274_206,
                4,
            ),
            (
                "r3k2r/8/3Q4/8/8/5q2/8/R3K2R b KQkq - 0 1",
                "Castling prevented",
                1_720_476,
                4,
            ),
            (
                "2K2r2/4P3/8/8/8/8/8/3k4 w - - 0 1",
                "Promote out of check",
                3_821_001,
                6,
            ),
            (
                "8/8/1P2K3/8/2n5/1q6/8/5k2 b - - 0 1",
                "Discovered check",
                1_004_658,
                5,
            ),
            (
                "4k3/1P6/8/8/8/8/K7/8 w - - 0 1",
                "Promote to give check",
                217_342,
                6,
            ),
            (
                "8/P1k5/K7/8/8/8/8/8 w - - 0 1",
                "Under promote to give check",
                92_683,
                6,
            ),
            ("K1k5/8/P7/8/8/8/8/8 w - - 0 1", "Self stalemate", 2_217, 6),
            (
                "8/k1P5/8/1K6/8/8/8/8 w - - 0 1",
                "Stalemate & checkmate #1",
                567_584,
                7,
            ),
            (
                "8/8/2k5/5q2/5n2/8/5K2/8 b - - 0 1",
                "Stalemate & checkmate #2",
                23_527,
                4,
            ),
        ];

        let mut failures = Vec::new();
        let mut passed = 0;
        let mut speeds = Vec::new();

        for (fen, desc, expected, depth) in PERFT_SUITE {
            println!("\nTesting: {} ({})", desc, fen);
            let mut board = Board::from_fen(fen);
            let start = Instant::now();
            let nodes = board.perft::<BULK>(depth);
            let duration = start.elapsed();

            let nodes_per_sec = if duration.as_micros() > 0 {
                (nodes as f64 / duration.as_micros() as f64) * 1_000_000.0
            } else {
                0.0
            };
            let mnps = nodes_per_sec / 1_000_000.0;
            speeds.push(mnps);

            if nodes == expected {
                println!(
                    "✓ {}: {} nodes (expected {}) - PASSED - {:.2} Mnps",
                    desc, nodes, expected, mnps
                );
                passed += 1;
            } else {
                println!(
                    "✗ {}: {} nodes (expected {}) - FAILED (difference: {}) - {:.2} Mnps",
                    desc,
                    nodes,
                    expected,
                    if nodes > expected {
                        format!("+{}", nodes - expected)
                    } else {
                        format!("-{}", expected - nodes)
                    },
                    mnps
                );
                failures.push((desc, fen, depth, expected, nodes));
            }
        }

        let total_tests = PERFT_SUITE.len();
        let avg_mnps: f64 = if !speeds.is_empty() {
            speeds.iter().sum::<f64>() / total_tests as f64
        } else {
            0.0
        };

        println!("\nTest Summary:");
        println!("Passed: {}/{}", passed, total_tests);
        println!("Failed: {}/{}", failures.len(), total_tests);
        println!("Average Speed: {:.2} Mnps", avg_mnps);

        if !failures.is_empty() {
            println!("\nFailed Tests:");
            for (desc, _, _, expected, got) in &failures {
                println!(
                    "- {} expected={}, got={}, diff={}",
                    desc,
                    expected,
                    got,
                    (*got as i64 - *expected as i64)
                );
            }
        }
    }
}
