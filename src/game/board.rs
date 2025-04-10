use super::{
    constants::{bishop_attacks, rook_attacks, KING_ATTACKS, KNIGHT_ATTACKS},
    moves::MoveKind,
    square::Square,
    zobrist::ZHash,
};
use crate::game::{
    bitboard::BitBoard,
    castle::CastlingRights,
    moves::*,
    piece::{Colour, Piece},
};

#[derive(Copy, Clone, Debug)]
pub struct Board {
    pub pieces: [BitBoard; 6],
    pub sides: [BitBoard; 2],

    pub piece_map: [Option<Piece>; Square::COUNT],

    pub side: Colour,
    pub castling_rights: CastlingRights,
    pub en_passant: Option<Square>,
    pub halfmoves: u8,
    pub hash: ZHash,
}

impl Board {
    pub fn new() -> Self {
        Board {
            pieces: [BitBoard::EMPTY; 6],
            sides: [BitBoard::EMPTY; 2],
            piece_map: [None; Square::COUNT],
            en_passant: None,
            castling_rights: CastlingRights::NONE,
            halfmoves: 0,
            side: Colour::White,
            hash: ZHash::NULL,
        }
    }

    pub fn default() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        self.piece_map[square.index()]
    }

    fn set_piece(&mut self, piece: Piece, square: Square) {
        let colour = piece.colour() as usize;
        self.sides[colour] = self.sides[colour].set_bit(square);
        self.pieces[piece.index()] = self.pieces[piece.index()].set_bit(square);
        self.piece_map[square.index()] = Some(piece);
        self.hash.hash_piece(piece, square);
    }

    fn remove_piece(&mut self, square: Square) {
        let piece = self.piece_at(square).expect("No piece in position given");
        let colour = piece.colour() as usize;

        self.sides[colour] = self.sides[colour].pop_bit(square);
        self.pieces[piece.index()] = self.pieces[piece.index()].pop_bit(square);
        self.piece_map[square.index()] = None;
        self.hash.hash_piece(piece, square);
    }

    pub fn occupied(&self) -> usize {
        (self.sides[Colour::White as usize] | self.sides[Colour::Black as usize]).count_bits()
            as usize
    }

    pub fn make_move(&mut self, m: Move) {
        let (src, dest) = (m.get_source(), m.get_dest());
        let src_piece = self.piece_at(src).expect("Invalid source piece");
        let move_type = m.get_type();
        let old_rights = self.castling_rights;

        if let Some(square) = self.en_passant {
            self.en_passant = None;
            self.hash.hash_enpassant(square);
        }

        if src_piece.is_pawn() || matches!(move_type, MoveKind::Capture) {
            self.halfmoves = 0
        } else {
            self.halfmoves += 1;
        }

        if src_piece.is_king() {
            if src_piece.colour() == Colour::White {
                let new_rights =
                    CastlingRights(old_rights.0 & !(CastlingRights::WK | CastlingRights::WQ));
                self.castling_rights = new_rights;
                self.hash.swap_castle(old_rights, new_rights);
            } else {
                let new_rights =
                    CastlingRights(old_rights.0 & !(CastlingRights::BK | CastlingRights::BQ));
                self.castling_rights = new_rights;
                self.hash.swap_castle(old_rights, new_rights);
            }
        } else if src_piece.is_rook() {
            let new_rights = match (src_piece.colour(), src.index()) {
                (Colour::White, 0) => CastlingRights(old_rights.0 & !CastlingRights::WQ), // a1
                (Colour::White, 7) => CastlingRights(old_rights.0 & !CastlingRights::WK), // h1
                (Colour::Black, 56) => CastlingRights(old_rights.0 & !CastlingRights::BQ), // a8
                (Colour::Black, 63) => CastlingRights(old_rights.0 & !CastlingRights::BK), // h8
                _ => old_rights,
            };
            if new_rights != old_rights {
                self.castling_rights = new_rights;
                self.hash.swap_castle(old_rights, new_rights);
            }
        }

        match move_type {
            MoveKind::Quiet | MoveKind::DoublePush => {
                self.remove_piece(src);
                self.set_piece(src_piece, dest);

                if matches!(move_type, MoveKind::DoublePush) {
                    let delta = src_piece.colour().forward();
                    self.en_passant = src.jump(0, delta);
                    self.hash.hash_enpassant(self.en_passant.unwrap());
                }
            }
            MoveKind::Capture => {
                self.remove_piece(dest);
                self.remove_piece(src);
                self.set_piece(src_piece, dest);
            }
            MoveKind::EnPassant => {
                let captured_pawn_square = dest
                    .jump(0, -src_piece.colour().forward())
                    .expect("Off the board en_passant");
                self.remove_piece(captured_pawn_square);
                self.remove_piece(src);
                self.set_piece(src_piece, dest);
            }
            MoveKind::Castle => {
                let is_kingside = dest.col() > src.col();
                let (rook_src_col, rook_dest_col) = if is_kingside { (7, 5) } else { (0, 3) };
                let row = src.row();
                let rook_src = Square::from_row_col(row, rook_src_col);
                let rook_dest = Square::from_row_col(row, rook_dest_col);
                let rook_piece = self.piece_at(rook_src).expect("Expected rook");

                self.remove_piece(src);
                self.remove_piece(rook_src);
                self.set_piece(src_piece, dest);
                self.set_piece(rook_piece, rook_dest);
            }
            _ => {
                #[cfg(debug_assertions)]
                assert!(move_type.is_promotion(), "Expected a promotion move");
                let promo_piece = move_type.get_promotion(src_piece.colour());
                self.remove_piece(src);
                if move_type.is_capture() {
                    self.remove_piece(dest);
                }
                self.set_piece(promo_piece, dest);
            }
        }

        self.side = !self.side;
        self.hash.hash_side();
        #[cfg(debug_assertions)]
        assert_eq!(self.hash, ZHash::new(self), "Hash mismatch after move");
    }

    fn generate_pseudo_moves(&self, side: Colour) -> Vec<Move> {
        let mut moves = Vec::with_capacity(64);
        let side_idx = side as usize;

        // Pawn moves
        let mut pawn_bb = self.pieces[Piece::WP.index()] & self.sides[side_idx];
        while pawn_bb != BitBoard::EMPTY {
            let src = pawn_bb.lsb();
            moves.extend(all_pawn_moves(
                src,
                if side == Colour::White {
                    Piece::WP
                } else {
                    Piece::BP
                },
                self,
            ));
            pawn_bb = pawn_bb.pop_bit(src);
        }

        // Knight moves
        let mut knight_bb = self.pieces[Piece::WN.index()] & self.sides[side_idx];
        while knight_bb != BitBoard::EMPTY {
            let src = knight_bb.lsb();
            moves.extend(all_knight_moves(src));
            knight_bb = knight_bb.pop_bit(src);
        }

        // Bishop moves
        let mut bishop_bb = self.pieces[Piece::WB.index()] & self.sides[side_idx];
        while bishop_bb != BitBoard::EMPTY {
            let src = bishop_bb.lsb();
            moves.extend(all_bishop_moves(src, self));
            bishop_bb = bishop_bb.pop_bit(src);
        }

        // Rook moves
        let mut rook_bb = self.pieces[Piece::WR.index()] & self.sides[side_idx];
        while rook_bb != BitBoard::EMPTY {
            let src = rook_bb.lsb();
            moves.extend(all_rook_moves(src, self));
            rook_bb = rook_bb.pop_bit(src);
        }

        // Queen moves
        let mut queen_bb = self.pieces[Piece::WQ.index()] & self.sides[side_idx];
        while queen_bb != BitBoard::EMPTY {
            let src = queen_bb.lsb();
            moves.extend(all_queen_moves(src, self));
            queen_bb = queen_bb.pop_bit(src);
        }

        // King moves
        let mut king_bb = self.pieces[Piece::WK.index()] & self.sides[side_idx];
        while king_bb != BitBoard::EMPTY {
            let src = king_bb.lsb();
            moves.extend(all_king_moves(src));
            king_bb = king_bb.pop_bit(src);
        }

        moves
    }

    pub fn generate_legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::new();
        let side = self.side;

        let pseudo_moves = self.generate_pseudo_moves(side);

        for m in pseudo_moves {
            if self.is_pseudo_legal(m) {
                let mut new_board = *self;
                new_board.make_move(m);

                if !new_board.is_attacked_by(new_board.king_square(side), !side) {
                    moves.push(m);
                }
            }
        }

        moves
    }

    fn is_pseudo_legal(&self, m: Move) -> bool {
        let src = m.get_source();
        let dest = m.get_dest();
        let move_type = m.get_type();

        let piece = self.piece_at(src).unwrap();
        let occupied = self.sides[Colour::White as usize] | self.sides[Colour::Black as usize];
        let opponent = self.sides[!self.side as usize];
        let forward = piece.colour().forward();

        match move_type {
            MoveKind::Quiet => !occupied.get_bit(dest),
            MoveKind::Capture => opponent.get_bit(dest),
            MoveKind::DoublePush => {
                let middle = src.jump(0, forward).unwrap();
                !occupied.get_bit(middle) && !occupied.get_bit(dest)
            }
            MoveKind::EnPassant => {
                self.en_passant == Some(dest) && opponent.get_bit(dest.jump(0, -forward).unwrap())
            }
            MoveKind::Castle => {
                let (rook_sq, king_pass, king_end, inter_squares) = match (self.side, dest) {
                    (Colour::White, d) if d == Square::from("g1") => (
                        Square::from("h1"),
                        Square::from("f1"),
                        Square::from("g1"),
                        BitBoard::WHITE_KING_CASTLE,
                    ),
                    (Colour::White, d) if d == Square::from("c1") => (
                        Square::from("a1"),
                        Square::from("d1"),
                        Square::from("c1"),
                        BitBoard::WHITE_QUEEN_CASTLE,
                    ),
                    (Colour::Black, d) if d == Square::from("g8") => (
                        Square::from("h8"),
                        Square::from("f8"),
                        Square::from("g8"),
                        BitBoard::BLACK_KING_CASTLE,
                    ),
                    (Colour::Black, d) if d == Square::from("c8") => (
                        Square::from("a8"),
                        Square::from("d8"),
                        Square::from("c8"),
                        BitBoard::BLACK_QUEEN_CASTLE,
                    ),
                    _ => return false,
                };

                let valid_rights = match (self.side, dest) {
                    (Colour::White, d) if d == Square::from("g1") => {
                        self.castling_rights.0 & CastlingRights::WK != 0
                    }
                    (Colour::White, d) if d == Square::from("c1") => {
                        self.castling_rights.0 & CastlingRights::WQ != 0
                    }
                    (Colour::Black, d) if d == Square::from("g8") => {
                        self.castling_rights.0 & CastlingRights::BK != 0
                    }
                    (Colour::Black, d) if d == Square::from("c8") => {
                        self.castling_rights.0 & CastlingRights::BQ != 0
                    }
                    _ => return false,
                };

                valid_rights
                    && inter_squares & occupied == BitBoard::EMPTY
                    && !self.is_attacked_by(self.king_square(self.side), !self.side)
                    && !self.is_attacked_by(king_pass, !self.side)
                    && !self.is_attacked_by(king_end, !self.side)
                    && self.piece_at(rook_sq)
                        == Some(if piece.colour() == Colour::White {
                            Piece::WR
                        } else {
                            Piece::BR
                        })
            }
            MoveKind::KnightPromotion
            | MoveKind::BishopPromotion
            | MoveKind::RookPromotion
            | MoveKind::QueenPromotion => !occupied.get_bit(dest),
            MoveKind::KnightCapPromo
            | MoveKind::BishopCapPromo
            | MoveKind::RookCapPromo
            | MoveKind::QueenCapPromo => opponent.get_bit(dest),
        }
    }

    /// Returns whether the given square is attacked by the given side or not,
    /// it uses sliding for bishop-queen and pawn, Obstruction difference with Infuehr improvement
    /// and precalculated bitboards for Knights and Kings
    pub fn is_attacked_by(&self, square: Square, attacker: Colour) -> bool {
        let idx = square.index();
        let enemy_side = self.sides[attacker as usize];

        // Knights
        if KNIGHT_ATTACKS[idx] & self.pieces[Piece::WN.index()] & enemy_side != BitBoard::EMPTY {
            return true;
        }

        // Kings
        if KING_ATTACKS[idx] & self.pieces[Piece::WK.index()] & enemy_side != BitBoard::EMPTY {
            return true;
        }

        // Rooks
        let occupied = self.sides[Colour::White as usize] | self.sides[Colour::Black as usize];
        let rook_attackers = self.pieces[Piece::WR.index()] | self.pieces[Piece::WQ.index()];
        if rook_attacks(occupied.0, idx) & rook_attackers & enemy_side != BitBoard::EMPTY {
            return true;
        }

        // Bishop
        let occupied = self.sides[Colour::White as usize] | self.sides[Colour::Black as usize];
        let bishop_attackers = self.pieces[Piece::WB.index()] | self.pieces[Piece::WQ.index()];
        if bishop_attacks(occupied.0, idx) & bishop_attackers & enemy_side != BitBoard::EMPTY {
            return true;
        }

        // Pawns
        let pawn_offsets = if attacker == Colour::White {
            [[-1, -1], [1, -1]]
        } else {
            [[-1, 1], [1, 1]]
        };
        for &[dr, df] in &pawn_offsets {
            if let Some(src) = square.jump(dr, df) {
                if let Some(piece) = self.piece_at(src) {
                    if piece.colour() == attacker && piece.is_pawn() {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn is_draw(&self) -> bool {
        if self.halfmoves >= 100 {
            return true;
        }

        if self.pieces[Piece::WP.index()]
            | self.pieces[Piece::WQ.index()]
            | self.pieces[Piece::WR.index()]
            == BitBoard::EMPTY
        {
            if (self.sides[Colour::White as usize] | self.sides[Colour::Black as usize])
                .count_bits()
                <= 3
            {
                return true;
            }

            if self.pieces[Piece::WN.index()] != BitBoard::EMPTY {
                return false;
            }

            let bishop_pos = self.pieces[Piece::WB.index()];
            return bishop_pos & BitBoard::WHITE_SQUARES == bishop_pos
                || bishop_pos & BitBoard::BLACK_SQUARES == bishop_pos;
        }

        false
    }

    pub fn king_square(&self, colour: Colour) -> Square {
        let king_bb = self.pieces[Piece::WK.index()] & self.sides[colour as usize];
        king_bb.lsb()
    }

    pub fn from_fen(state: &str) -> Self {
        let fen: Vec<&str> = state.split_whitespace().take(6).collect();

        if fen.len() != 6 {
            panic!("Invalid input FEN string");
        }

        let board_layout = fen[0];
        let mut board = Board::new();
        let (mut row, mut col): (u8, u8) = (7, 0);
        let mut tokens = 0;

        for token in board_layout.chars() {
            match token {
                '/' => {
                    if tokens != 8 {
                        panic!("Invalid number of positions in FEN");
                    }

                    row -= 1;
                    col = 0;
                    tokens = 0;
                }
                '1'..='8' => {
                    let empty_pos = token.to_digit(10).expect("Not a number") as u8;
                    col += empty_pos;
                    tokens += empty_pos;
                }
                _ => {
                    board.set_piece(
                        Piece::from_fen(token),
                        Square::from_row_col(row as usize, col as usize),
                    );

                    col += 1;
                    tokens += 1;
                }
            }
        }

        board.side = match fen[1] {
            "w" => Colour::White,
            "b" => Colour::Black,
            _ => unreachable!(),
        };

        board.castling_rights = CastlingRights::from(fen[2]);

        board.en_passant = match fen[3] {
            "-" => None,
            _ => Some(Square::from(fen[3])),
        };

        board.halfmoves = fen[4].parse::<u8>().unwrap();
        board.hash = ZHash::new(&board);

        board
    }

    #[allow(dead_code)]
    pub fn draw_board(&self) {
        print!("\x1B[2J\x1B[1;1H");
        println!("\r  a b c d e f g h\r");
        println!(" ┌────────────────┐\r");

        for row in (0..8).rev() {
            print!("{}│", row + 1);
            for col in 0..8 {
                let square = Square::from_row_col(row, col);
                let bg_colour = if (row + col) % 2 == 0 {
                    "\x1b[48;2;240;217;181m"
                } else {
                    "\x1b[48;2;181;136;99m"
                };

                match self.piece_map[square.index()] {
                    Some(piece) => match piece.colour() {
                        Colour::White => print!("{bg_colour}\x1b[38;2;255;255;255m{piece} \x1b[0m"),
                        Colour::Black => print!("{bg_colour}\x1b[38;2;0;0;0m{piece} \x1b[0m"),
                    },
                    None => print!("{bg_colour}  \x1b[0m"),
                }
            }

            println!("│\r");
        }

        println!(" └────────────────┘\r");
    }
}

// For debugging
impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  a b c d e f g h")?;
        writeln!(f, " ┌────────────────┐")?;

        for row in (0..8).rev() {
            write!(f, "{}│", row + 1)?;
            for col in 0..8 {
                let square = Square::from_row_col(row, col);
                match self.piece_map[square.index()] {
                    Some(piece) => write!(f, "{} ", piece)?,
                    None => write!(f, "  ")?,
                }
            }
            writeln!(f, "│")?;
        }

        writeln!(f, " └────────────────┘")?;
        Ok(())
    }
}
