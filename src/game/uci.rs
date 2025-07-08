use crate::engine::network::EvalTable;
use crate::engine::search::find_best_move;
use crate::engine::tables::SearchData;
use crate::game::piece::Colour;
use std::io::BufRead;
use std::{env, time::Instant};

use super::{
    board::Board,
    moves::{Move, MoveKind},
    perft::BULK,
    square::Square,
};

const NAME: &str = "Oxide";
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Time Control constants
const MAX_TIME: u128 = 180000;
const DEFAULT: f64 = 4000.0;

pub struct UCIEngine {
    board: Board,
    pub data: SearchData,
}

impl UCIEngine {
    pub fn new() -> Self {
        UCIEngine {
            board: Board::default(),
            data: SearchData::new(),
        }
    }

    pub fn run(&mut self) {
        let stdin = std::io::stdin();

        for line in stdin.lock().lines() {
            let line = line.unwrap();
            if line.is_empty() {
                continue;
            }

            self.process_command(&line);
        }
    }

    fn process_command(&mut self, command: &str) {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "uci" => {
                println!("id name {NAME} {VERSION}");
                println!("id author {AUTHOR}");
                println!("uciok");
            }
            "ucinewgame" => {
                self.board = Board::default();
                self.data.clear();
            }
            "isready" => {
                println!("readyok");
            }
            "position" => {
                self.parse_position(&parts[1..]);
            }
            "perft" => self.run_perft(&parts[1..]),
            "go" => {
                self.go(&parts[1..]);
            }
            "quit" => {
                std::process::exit(0);
            }
            _ => println!("Unexpected command"),
        }
    }

    fn parse_position(&mut self, args: &[&str]) {
        let mut board = if args[0] == "startpos" {
            Board::default()
        } else if args[0] == "fen" {
            let fen_end = args
                .iter()
                .position(|&x| x == "moves")
                .unwrap_or(args.len());
            let fen = args[1..fen_end].join(" ");
            Board::from_fen(&fen)
        } else {
            return;
        };

        self.data.clear();

        let moves_start = args.iter().position(|&x| x == "moves");
        if let Some(start) = moves_start {
            for move_str in &args[start + 1..] {
                self.data.push(self.board.hash.0);
                let m = self.parse_move(&board, move_str);
                board.make_move(m);
            }
        }

        self.board = board;
    }

    fn go(&mut self, args: &[&str]) {
        self.data.tt.tt.clear(); // TODO:
        self.data.cache = EvalTable::default();
        let mut depth: Option<u8> = None;
        let mut wtime: Option<usize> = None;
        let mut btime: Option<usize> = None;
        let mut winc: Option<usize> = None;
        let mut binc: Option<usize> = None;
        let mut moves_left: Option<f64> = None;
        let mut i = 0;

        while i < args.len() {
            match args[i] {
                "depth" if i + 1 < args.len() => {
                    i += 1;
                    depth = args[i].parse().ok();
                }
                "wtime" if i + 1 < args.len() => {
                    i += 1;
                    wtime = args[i].parse().ok();
                }
                "btime" if i + 1 < args.len() => {
                    i += 1;
                    btime = args[i].parse().ok();
                }
                "winc" if i + 1 < args.len() => {
                    i += 1;
                    winc = args[i].parse().ok();
                }
                "binc" if i + 1 < args.len() => {
                    i += 1;
                    binc = args[i].parse().ok();
                }
                "movestogo" if i + 1 < args.len() => {
                    i += 1;
                    moves_left = args[i].parse().ok()
                }
                _ => {}
            }
            i += 1;
        }

        let time_left = match self.board.side {
            Colour::White => wtime,
            Colour::Black => btime,
        };
        let time_incr = match self.board.side {
            Colour::White => binc,
            Colour::Black => winc,
        };

        let play_time = if let Some(t) = time_left {
            (if let Some(inc) = time_incr {
                (t / 30 + 3 * inc / 4) as u128
            } else {
                (t as f64 / moves_left.unwrap_or(30.0)
                    * match self.board.halfmoves {
                        0..=10 => 0.6,
                        11..=30 => 1.1,
                        31..=50 => 1.25,
                        _ => 0.9,
                    }) as u128
            })
            .min((t as f64 * 0.95) as u128)
        } else {
            (DEFAULT
                * match self.board.halfmoves {
                    0..=10 => 0.64,
                    11..=30 => 1.0,
                    _ => 1.25,
                }) as u128
        }
        .min(MAX_TIME);

        find_best_move(&self.board, depth, play_time, &mut self.data);
        println!("bestmove {}", self.data.best_move);
    }

    fn parse_move(&self, board: &Board, move_str: &str) -> Move {
        let src = Square::from(&move_str[0..2]);
        let dest = Square::from(&move_str[2..4]);
        let promotion = move_str.get(4..5);

        let moves = board.generate_legal_moves::<true>();
        for m in moves {
            if m.get_source() == src && m.get_dest() == dest {
                if let Some(promo_char) = promotion {
                    let promo_piece = match promo_char {
                        "q" => MoveKind::QueenPromotion,
                        "r" => MoveKind::RookPromotion,
                        "b" => MoveKind::BishopPromotion,
                        "n" => MoveKind::KnightPromotion,
                        _ => continue,
                    };
                    if m.get_type() == promo_piece || m.get_type() == promo_piece.with_capture() {
                        return m;
                    }
                } else if !m.get_type().is_promotion() {
                    return m;
                }
            }
        }
        Move::default() // Fallback
    }

    fn run_perft(&mut self, args: &[&str]) {
        let depth = if args.is_empty() {
            8
        } else {
            args[0].parse().unwrap_or(8)
        };

        let start = Instant::now();
        let total_nodes = self.board.perft::<BULK>(depth);
        let total_duration = start.elapsed();

        let nodes_per_sec = if total_duration.as_micros() > 0 {
            (total_nodes as f64 / total_duration.as_micros() as f64) * 1_000_000.0
        } else {
            0.0
        };

        println!(
            "info string Total: {} nodes in {:.3}s - {:.2} Mnps",
            total_nodes,
            total_duration.as_secs_f64(),
            nodes_per_sec / 1_000_000.0
        );
    }
}

impl MoveKind {
    fn with_capture(self) -> Self {
        match self {
            MoveKind::KnightPromotion => MoveKind::KnightCapPromo,
            MoveKind::BishopPromotion => MoveKind::BishopCapPromo,
            MoveKind::RookPromotion => MoveKind::RookCapPromo,
            MoveKind::QueenPromotion => MoveKind::QueenCapPromo,
            _ => self,
        }
    }
}
