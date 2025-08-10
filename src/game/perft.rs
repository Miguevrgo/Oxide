use crate::game::board::Board;
use std::time::Instant;

impl Board {
    fn non_bulk_perft<const ROOT: bool>(&self, depth: usize) -> usize {
        if depth == 0 {
            return 1;
        }

        let mut total = 0;
        let moves = self.generate_pseudo_moves::<true>(self.side);
        for m in &moves {
            if !self.is_legal(m) {
                continue;
            }

            if depth == 1 {
                total += 1;
            } else {
                let mut new = *self;
                new.make_move(m);
                let count = new.non_bulk_perft::<false>(depth - 1);

                total += count;

                if ROOT {
                    println!("{m}: {count}")
                }
            }
        }

        total
    }

    pub fn perft(&self, depth: usize) -> usize {
        let start = Instant::now();
        let total_nodes = self.non_bulk_perft::<true>(depth);
        let duration = start.elapsed().as_millis() as usize;
        let perft = total_nodes / duration.max(1) / 1_000;
        println!("\n{total_nodes} nodes in {duration:?} - {perft} Mn/s");

        total_nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perft_suite() {
        #[rustfmt::skip]
        const PERFT_SUITE: [(&str, &str, usize, usize); 19] = [
            ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", "Startpos", 119060324, 6),
            ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", "Kiwipete", 193690690, 5),
            ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", "Rook and pawns Pos 3 CPW", 11030083, 6),
            ("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", "Pos 4 CPW", 15833292, 5),
            ("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", "Pos 5 CPW", 89941194, 5),
            ("8/8/4k3/8/2p5/8/B2P2K1/8 w - - 0 1", "Illegal ep move #1", 14047573, 7),
            ("3k4/3p4/8/K1P4r/8/8/8/8 b - - 0 1", "Illegal ep move #2", 20757544, 7),
            ("8/8/1k6/2b5/2pP4/8/5K2/8 b - d3 0 1", "Ep capture checks opponent", 21190412, 7),
            ("5k2/8/8/8/8/8/8/4K2R w K - 0 1", "Short castling gives check", 12762196, 7),
            ("3k4/8/8/8/8/8/8/R3K3 w Q - 0 1", "Long castling gives check", 91628014, 8),
            ("r3k2r/1b4bq/8/8/8/8/7B/R3K2R w KQkq - 0 1", "Castle rights", 31912360, 5),
            ("r3k2r/8/3Q4/8/8/5q2/8/R3K2R b KQkq - 0 1", "Castling prevented", 58773923, 5),
            ("2K2r2/4P3/8/8/8/8/8/3k4 w - - 0 1", "Promote out of check", 60651209, 7),
            ("8/8/1P2K3/8/2n5/1q6/8/5k2 b - - 0 1", "Discovered check", 6334638, 6),
            ("4k3/1P6/8/8/8/8/K7/8 w - - 0 1", "Promote to give check", 217342, 6),
            ("8/P1k5/K7/8/8/8/8/8 w - - 0 1", "Under promote to give check", 8110830, 8),
            ("K1k5/8/P7/8/8/8/8/8 w - - 0 1", "Self stalemate", 5966690, 10),
            ("8/k1P5/8/1K6/8/8/8/8 w - - 0 1", "Stalemate & checkmate #1", 2518905, 8),
            ("8/8/2k5/5q2/5n2/8/5K2/8 b - - 0 1", "Stalemate & checkmate #2", 3114998, 6),
        ];

        let mut failures = Vec::new();
        let mut passed = 0;
        let mut speeds = Vec::new();

        for (fen, desc, expected, depth) in PERFT_SUITE {
            println!("\nTesting: {desc} ({fen})");
            let board = Board::from_fen(fen);
            let start = Instant::now();
            let nodes = board.perft(depth);
            let duration = start.elapsed();

            let nodes_per_sec = if duration.as_micros() > 0 {
                (nodes as f64 / duration.as_micros() as f64) * 1_000_000.0
            } else {
                0.0
            };
            let mnps = nodes_per_sec / 1_000_000.0;
            speeds.push(mnps);

            if nodes == expected {
                println!("✓ {desc}: {nodes} nodes (expected {expected}) - PASSED - {mnps:.2} Mnps");
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
        println!("Passed: {passed}/{total_tests}");
        println!("Failed: {}/{}", failures.len(), total_tests);
        println!("Average Speed: {avg_mnps:.2} Mnps");

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
