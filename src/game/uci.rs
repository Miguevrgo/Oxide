use crate::engine::network::EvalTable;
use crate::engine::search::{find_best_move, MAX_DEPTH};
use crate::engine::tables::SearchData;
use crate::game::piece::Colour;
use std::env;
use std::io::BufRead;

use super::{
    board::Board,
    moves::{Move, MoveKind},
    square::Square,
};

const NAME: &str = "Oxide";
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Time Control constants
const MAX_TIME: u128 = 180000;

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
                println!("option name Hash type spin default 32 min 1 max 4096");
                println!("option name Threads type spin default 1 min 1 max 1");
                println!("uciok");
            }
            "ucinewgame" => {
                self.board = Board::default();
                self.data.clear();
                self.data.tt.clear();
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
            "setoption" => {
                if parts.len() >= 5 && parts[1] == "name" {
                    match parts[2] {
                        "Hash" if parts[3] == "value" => {
                            if let Ok(mb) = parts[4].parse() {
                                if mb > 0 {
                                    self.data.resize_tt(mb);
                                }
                            }
                        }
                        "Threads" if parts[3] == "value" => {
                            if let Ok(n) = parts[4].parse::<u8>() {
                                if n != 1 {
                                    println!("Only one thread supported!")
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            "eval" => {
                println!("eval: {}cp", self.board.evaluate(&mut EvalTable::default()));
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
                self.data.stack.push(board.hash.0);
                let m = self.parse_move(&board, move_str);
                board.make_move(m);
            }
        }

        self.board = board;
    }

    fn go(&mut self, args: &[&str]) {
        self.data.tt.inc_age();
        self.data.cache = EvalTable::default();
        let mut depth: u8 = 64;
        let mut wtime: Option<usize> = None;
        let mut btime: Option<usize> = None;
        let mut winc: Option<usize> = None;
        let mut binc: Option<usize> = None;
        let mut moves_left: Option<f64> = None;
        let mut movetime: Option<u128> = None;

        let mut i = 0;
        while i + 1 < args.len() {
            let value = args[i];
            i += 1;
            match value {
                "depth" => depth = args[i].parse().unwrap_or(MAX_DEPTH).clamp(1, 64),
                "wtime" => wtime = args[i].parse().ok(),
                "btime" => btime = args[i].parse().ok(),
                "winc" => winc = args[i].parse().ok(),
                "binc" => binc = args[i].parse().ok(),
                "movestogo" => moves_left = args[i].parse().ok(),
                "movetime" => movetime = args[i].parse().ok(),
                _ => i -= 1,
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

        self.data.time_tp = if let Some(t) = time_left {
            (if let Some(inc) = time_incr {
                (t / 20 + 4 * inc / 5) as u128
            } else {
                (t as f64 / moves_left.unwrap_or(30.0)
                    * match self.board.halfmoves {
                        0..=10 => 0.6,
                        11..=30 => 1.1,
                        31..=50 => 1.35,
                        _ => 1.0,
                    }) as u128
            })
            .min((t as f64 * 0.95) as u128)
        } else if let Some(time_tm) = movetime {
            time_tm
        } else {
            MAX_TIME
        }
        .min(MAX_TIME);

        find_best_move(&self.board, depth, &mut self.data);
        println!("bestmove {}", self.data.best_move);
    }

    fn parse_move(&self, board: &Board, move_str: &str) -> Move {
        let src = Square::from(&move_str[0..2]);
        let dest = Square::from(&move_str[2..4]);
        let promotion = move_str.get(4..5);

        let moves = board.generate_pseudo_moves::<true, true>();
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
            7
        } else {
            args[0].parse().unwrap_or(8)
        };

        self.board.perft(depth);
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
