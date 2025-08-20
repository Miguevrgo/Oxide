use crate::engine::network::{EvalTable, Network};
use crate::game::constants::queen_attacks;
use crate::game::{
    bitboard::BitBoard,
    castle::CastlingRights,
    constants::{bishop_attacks, rook_attacks, KING_ATTACKS, KNIGHT_ATTACKS, PIECE_VALUES},
    moves::{Move, MoveKind},
    piece::{Colour, Piece},
    square::Square,
    zobrist::ZHash,
};

use super::constants::{between, pinned_moves, PAWN_ATTACKS};
use super::moves::MoveList;

#[derive(Copy, Clone, Debug)]
pub struct Board {
    pub pieces: [BitBoard; 6],
    pub sides: [BitBoard; 2],

    piece_map: [Piece; Square::COUNT],

    pub side: Colour,
    pub castling_rights: CastlingRights,
    pub en_passant: Option<Square>,
    pub halfmoves: u8,
    pub hash: ZHash,
    pub checkers: BitBoard,
    pub threats: BitBoard,
    pub pinned: BitBoard,
}

impl Board {
    pub fn new() -> Self {
        Board {
            pieces: [BitBoard::EMPTY; 6],
            sides: [BitBoard::EMPTY; 2],
            piece_map: [Piece::Empty; Square::COUNT],
            en_passant: None,
            castling_rights: CastlingRights::NONE,
            halfmoves: 0,
            side: Colour::White,
            hash: ZHash::NULL,
            checkers: BitBoard::EMPTY,
            threats: BitBoard::EMPTY,
            pinned: BitBoard::EMPTY,
        }
    }

    pub fn default() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    pub fn piece_at(&self, square: Square) -> Piece {
        self.piece_map[square.index()]
    }

    pub fn capture_piece(&self, m: Move) -> Piece {
        if m.get_type() == MoveKind::EnPassant {
            match self.side {
                Colour::Black => Piece::WP,
                Colour::White => Piece::BP,
            }
        } else {
            self.piece_at(m.get_dest())
        }
    }

    fn set_piece(&mut self, piece: Piece, square: Square) {
        let colour = piece.colour() as usize;
        let bit = 1u64 << square.index();
        self.sides[colour] ^= bit;
        self.pieces[piece.index()] ^= bit;
        self.piece_map[square.index()] = piece;
        self.hash.hash_piece(piece, square);
    }

    fn remove_piece(&mut self, square: Square) {
        let piece = self.piece_at(square);
        let colour = piece.colour() as usize;
        let bit = 1u64 << square.index();

        self.sides[colour] ^= bit;
        self.pieces[piece.index()] ^= bit;
        self.piece_map[square.index()] = Piece::Empty;
        self.hash.hash_piece(piece, square);
    }

    pub fn make_move(&mut self, m: Move) {
        let (src, dest) = (m.get_source(), m.get_dest());
        let src_piece = self.piece_at(src);
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
                    self.en_passant = Some(src.shift::<8>(src_piece.colour()));
                    self.hash.hash_enpassant(self.en_passant.unwrap());
                }
            }
            MoveKind::Capture => {
                self.remove_piece(dest);
                self.remove_piece(src);
                self.set_piece(src_piece, dest);
            }
            MoveKind::EnPassant => {
                let captured_pawn_square = dest.shift::<8>(!src_piece.colour());
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
                let rook_piece = self.piece_at(rook_src);

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
        self.calculate_threats();
        self.pinned_and_checkers();
    }

    /// Updates the threats bitboard with the current squares under attack by any piece of the
    /// opposite board colour
    pub fn calculate_threats(&mut self) {
        let attacker = !self.side as usize;
        self.threats = BitBoard::EMPTY;
        let occ = (self.sides[0] | self.sides[1]) ^ self.king_square(self.side as usize).to_board();

        // Pawn attacks
        let mut pawns = self.pieces[Piece::WP.index()] & self.sides[attacker];
        while pawns != BitBoard::EMPTY {
            let sq = pawns.lsb();
            self.threats |= PAWN_ATTACKS[attacker][sq.index()];
            pawns = pawns.pop_bit(sq);
        }

        // Rooks and queens (Orthogonal)
        let mut rooks = (self.pieces[Piece::WR.index()] | self.pieces[Piece::WQ.index()])
            & self.sides[attacker];
        while rooks != BitBoard::EMPTY {
            let sq = rooks.lsb();
            self.threats |= rook_attacks(occ.0, sq.index());
            rooks = rooks.pop_bit(sq);
        }

        // Bishops and queens (diagonals)
        let mut bishops = (self.pieces[Piece::WB.index()] | self.pieces[Piece::WQ.index()])
            & self.sides[attacker];
        while bishops != BitBoard::EMPTY {
            let sq = bishops.lsb();
            self.threats |= bishop_attacks(occ.0, sq.index());
            bishops = bishops.pop_bit(sq);
        }

        // Knight attacks (jumpers)
        let mut knights = self.pieces[Piece::WN.index()] & self.sides[attacker];
        while knights != BitBoard::EMPTY {
            let sq = knights.lsb();
            self.threats |= KNIGHT_ATTACKS[sq.index()];
            knights = knights.pop_bit(sq);
        }

        // King attacks
        let king_sq = self.king_square(attacker).index();
        self.threats |= KING_ATTACKS[king_sq];
    }

    /// Updates the pinned and checkers bitboards to include all of current board
    /// side pieces which are pinned and all enemy pieces which are currently providing
    /// a check
    pub fn pinned_and_checkers(&mut self) {
        self.pinned = BitBoard::EMPTY;
        let attacker = !self.side as usize;
        let king_sq = self.king_square(self.side as usize);
        let occ = self.sides[0] | self.sides[1];

        self.checkers = (KNIGHT_ATTACKS[king_sq.index()]
            & self.pieces[Piece::WN.index()]
            & self.sides[attacker])
            | (PAWN_ATTACKS[self.side as usize][king_sq.index()]
                & self.pieces[Piece::WP.index()]
                & self.sides[attacker]);

        let mut sliders_attacks = ((self.pieces[Piece::WB.index()]
            | self.pieces[Piece::WQ.index()])
            & self.sides[attacker]
            & bishop_attacks(BitBoard::EMPTY.0, king_sq.index()))
            | ((self.pieces[Piece::WR.index()] | self.pieces[Piece::WQ.index()])
                & self.sides[attacker]
                & rook_attacks(BitBoard::EMPTY.0, king_sq.index()));

        while sliders_attacks != BitBoard::EMPTY {
            let sq = sliders_attacks.lsb();
            let blockers = between(sq, king_sq) & occ;
            if blockers == BitBoard::EMPTY {
                self.checkers |= sq.to_board();
            } else if blockers.count_bits() == 1 {
                self.pinned |= blockers & self.sides[self.side as usize];
            }
            sliders_attacks = sliders_attacks.pop_bit(sq);
        }
    }

    /// Checks if the given castle is legal by checking castling_rights, checks, that there
    /// are no pieces in between and passing squares are not threatened.
    pub fn is_castle_legal(&self, dest: Square) -> bool {
        let (rook_sq, king_pass, king_end, inter_squares, right_bit) = match (self.side, dest) {
            (Colour::White, d) if d == Square::from("g1") => (
                Square::from("h1"),
                Square::from("f1"),
                Square::from("g1"),
                BitBoard::WHITE_KING_CASTLE,
                CastlingRights::WK,
            ),
            (Colour::White, d) if d == Square::from("c1") => (
                Square::from("a1"),
                Square::from("d1"),
                Square::from("c1"),
                BitBoard::WHITE_QUEEN_CASTLE,
                CastlingRights::WQ,
            ),
            (Colour::Black, d) if d == Square::from("g8") => (
                Square::from("h8"),
                Square::from("f8"),
                Square::from("g8"),
                BitBoard::BLACK_KING_CASTLE,
                CastlingRights::BK,
            ),
            (Colour::Black, d) if d == Square::from("c8") => (
                Square::from("a8"),
                Square::from("d8"),
                Square::from("c8"),
                BitBoard::BLACK_QUEEN_CASTLE,
                CastlingRights::BQ,
            ),
            _ => return false,
        };

        let occ = self.sides[Colour::White as usize] | self.sides[Colour::Black as usize];
        let rights_ok = self.castling_rights.0 & right_bit != 0;
        let path_clear = inter_squares & occ == BitBoard::EMPTY;
        if !(path_clear && rights_ok) {
            return false;
        }
        let safe = self.checkers == BitBoard::EMPTY
            && (king_pass.to_board() | king_end.to_board()) & self.threats == BitBoard::EMPTY;
        safe && self.piece_at(rook_sq)
            == if self.side == Colour::White {
                Piece::WR
            } else {
                Piece::BR
            }
    }

    pub fn generate_pseudo_moves<const QUIET: bool>(&self, side: Colour) -> MoveList {
        let mut moves = MoveList::default();
        let side_idx = side as usize;
        let occ = self.sides[Colour::White as usize] | self.sides[Colour::Black as usize];

        // King moves
        self.all_king_moves::<QUIET>(side, occ.0, &mut moves);

        // If there is more than 1 checker, the only possible move comes from the king
        if self.checkers.count_bits() > 1 {
            return moves;
        }

        // Pawn moves
        self.all_pawn_moves::<QUIET>(side, occ, &mut moves);

        // Knights
        self.all_knight_moves::<QUIET>(side, occ, &mut moves);

        // Bishop moves
        let mut bishop_bb = self.pieces[Piece::WB.index()] & self.sides[side_idx];
        while bishop_bb != BitBoard::EMPTY {
            let src = bishop_bb.lsb();
            self.all_slider_moves::<QUIET>(src, occ.0, bishop_attacks, &mut moves);
            bishop_bb = bishop_bb.pop_bit(src);
        }

        // Rook moves
        let mut rook_bb = self.pieces[Piece::WR.index()] & self.sides[side_idx];
        while rook_bb != BitBoard::EMPTY {
            let src = rook_bb.lsb();
            self.all_slider_moves::<QUIET>(src, occ.0, rook_attacks, &mut moves);
            rook_bb = rook_bb.pop_bit(src);
        }

        // Queen moves
        let mut queen_bb = self.pieces[Piece::WQ.index()] & self.sides[side_idx];
        while queen_bb != BitBoard::EMPTY {
            let src = queen_bb.lsb();
            self.all_slider_moves::<QUIET>(src, occ.0, queen_attacks, &mut moves);
            queen_bb = queen_bb.pop_bit(src);
        }

        moves
    }

    /// Returns wether the given move is legal or not by checking if the king would end in check after
    /// the move
    pub fn is_legal(&self, m: Move) -> bool {
        let src = m.get_source();
        let dest = m.get_dest();
        let mov_piece = self.piece_at(src);
        let king_pos = self.king_square(self.side as usize);

        if m.get_type() == MoveKind::EnPassant {
            let captured_pawn_sq = dest.shift::<8>(!self.side); // !self.side
            let occ = (self.sides[0] | self.sides[1])
                ^ src.to_board()
                ^ dest.to_board()
                ^ captured_pawn_sq.to_board();
            let diagonal_pieces = (self.pieces[Piece::WB.index()] | self.pieces[Piece::WQ.index()])
                & self.sides[!self.side as usize];
            let orthogonal_pieces = (self.pieces[Piece::WR.index()]
                | self.pieces[Piece::WQ.index()])
                & self.sides[!self.side as usize];
            return (bishop_attacks(occ.0, king_pos.index()) & diagonal_pieces == BitBoard::EMPTY)
                && (rook_attacks(occ.0, king_pos.index()) & orthogonal_pieces == BitBoard::EMPTY);
        }

        if mov_piece.is_king() {
            return !self.threats.contains(dest);
        }

        if self.pinned.contains(src) && !pinned_moves(king_pos, src).contains(dest) {
            return false;
        }

        match self.checkers.count_bits() {
            0 => true, // No checks
            1 => {
                // If the move is going to solve the only check
                let checker = self.checkers.lsb();
                dest == checker || between(king_pos, checker).contains(dest)
            }
            // As the move does not come from the king and there is more than one check, it cannot
            // be stopped
            _ => false,
        }
    }

    pub fn in_check(&self) -> bool {
        self.is_attacked_by(self.king_square(self.side as usize), !self.side)
    }

    pub fn make_null_move(&mut self) {
        self.side = !self.side;
        self.hash.hash_side();

        if let Some(sq) = self.en_passant {
            self.hash.hash_enpassant(sq);
            self.en_passant = None;
        }

        self.calculate_threats();
        self.pinned_and_checkers();
    }

    /// Returns whether the given square is attacked by the given side or not,
    /// it uses sliding for bishop-queen and pawn, Obstruction difference with Infuehr improvement
    /// and precalculated bitboards for Knights and Kings
    pub fn is_attacked_by(&self, square: Square, attacker: Colour) -> bool {
        let idx = square.index();
        let enemy_side = self.sides[attacker as usize];
        let occ = self.sides[Colour::White as usize] | self.sides[Colour::Black as usize];

        ((KNIGHT_ATTACKS[idx] & self.pieces[Piece::WN.index()])
            | (KING_ATTACKS[idx] & self.pieces[Piece::WK.index()])
            | (PAWN_ATTACKS[!attacker as usize][idx] & self.pieces[Piece::WP.index()])
            | (rook_attacks(occ.0, idx)
                & (self.pieces[Piece::WR.index()] | self.pieces[Piece::WQ.index()]))
            | (bishop_attacks(occ.0, idx)
                & (self.pieces[Piece::WB.index()] | self.pieces[Piece::WQ.index()])))
            & enemy_side
            != BitBoard::EMPTY
    }

    pub fn is_king_pawn(&self) -> bool {
        let occ = self.sides[self.side as usize];
        let pawn_king = self.pieces[Piece::WP.index()] | self.pieces[Piece::WK.index()];
        occ ^ (occ & pawn_king) == BitBoard::EMPTY
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

    pub fn king_square(&self, colour: usize) -> Square {
        let king_bb = self.pieces[Piece::WK.index()] & self.sides[colour];
        king_bb.lsb()
    }

    pub fn evaluate(&self, cache: &mut EvalTable) -> i32 {
        let white_king_sq = self.king_square(Colour::White as usize).index();
        let black_king_sq = self.king_square(Colour::Black as usize).index();

        let wbucket = Network::get_bucket::<0>(white_king_sq);
        let bbucket = Network::get_bucket::<1>(black_king_sq);

        let entry = &mut cache.table[wbucket][bbucket];

        let mut addf = [[0u16; 32]; 2];
        let mut subf = [[0u16; 32]; 2];
        let (adds, subs) = self.fill_diff(
            &entry.bbs,
            &mut addf,
            &mut subf,
            white_king_sq,
            black_king_sq,
        );

        entry.white.update_multi(&addf[0][..adds], &subf[0][..subs]);
        entry.black.update_multi(&addf[1][..adds], &subf[1][..subs]);

        entry.bbs = [
            self.sides[Colour::White as usize].0,
            self.sides[Colour::Black as usize].0,
            self.pieces[Piece::WP.index()].0,
            self.pieces[Piece::WN.index()].0,
            self.pieces[Piece::WB.index()].0,
            self.pieces[Piece::WR.index()].0,
            self.pieces[Piece::WQ.index()].0,
            self.pieces[Piece::WK.index()].0,
        ];

        let eval = match self.side {
            Colour::White => Network::out(&entry.white, &entry.black),
            Colour::Black => Network::out(&entry.black, &entry.white),
        };

        self.scale(eval)
    }

    fn fill_diff(
        &self,
        bbs: &[u64; 8],
        add_feats: &mut [[u16; 32]; 2],
        sub_feats: &mut [[u16; 32]; 2],
        white_king_sq: usize,
        black_king_sq: usize,
    ) -> (usize, usize) {
        let mut adds = 0;
        let mut subs = 0;

        let wflip = if white_king_sq % 8 > 3 { 7 } else { 0 };
        let bflip = if black_king_sq % 8 > 3 { 7 } else { 0 } ^ 56;

        for side in [Colour::White as usize, Colour::Black as usize] {
            let old_boys = bbs[side];
            let new_boys = self.sides[side].0;

            for (piece, &old_bb) in bbs[2..8].iter().enumerate() {
                let old_bb = old_bb & old_boys;
                let new_bb = self.pieces[piece].0 & new_boys;

                let wbase = Network::get_base_index::<0>(side, piece, white_king_sq) as u16;
                let bbase = Network::get_base_index::<1>(side, piece, black_king_sq) as u16;

                let mut add_diff = BitBoard(new_bb & !old_bb);
                while add_diff != BitBoard::EMPTY {
                    let sq = add_diff.lsb();
                    let sq_idx = sq.index() as u16;
                    add_feats[0][adds] = wbase + (sq_idx ^ wflip);
                    add_feats[1][adds] = bbase + (sq_idx ^ bflip);
                    adds += 1;
                    add_diff = add_diff.pop_bit(sq);
                }

                let mut sub_diff = BitBoard(old_bb & !new_bb);
                while sub_diff != BitBoard::EMPTY {
                    let sq = sub_diff.lsb();
                    let sq_idx = sq.index() as u16;
                    sub_feats[0][subs] = wbase + (sq_idx ^ wflip);
                    sub_feats[1][subs] = bbase + (sq_idx ^ bflip);
                    subs += 1;
                    sub_diff = sub_diff.pop_bit(sq);
                }
            }
        }

        (adds, subs)
    }

    pub fn scale(&self, eval: i32) -> i32 {
        let mat = 700
            + (self.pieces[Piece::WN.index()].count_bits() as i32
                * PIECE_VALUES[Piece::WN.index()]
                + self.pieces[Piece::WB.index()].count_bits() as i32
                    * PIECE_VALUES[Piece::WB.index()]
                + self.pieces[Piece::WR.index()].count_bits() as i32
                    * PIECE_VALUES[Piece::WR.index()]
                + self.pieces[Piece::WQ.index()].count_bits() as i32
                    * PIECE_VALUES[Piece::WQ.index()])
                / 32;

        eval * mat / 1024
    }

    /// Static exchange evaluation method, it does not check wether
    /// or not the move is a capture as it is only used in move_score
    /// for capture moves
    pub fn see(&self, m: Move, threshold: i32) -> bool {
        let src = m.get_source();
        let dest = m.get_dest();
        let mt = m.get_type();
        let mut next_piece = None;

        let mut score = if mt == MoveKind::EnPassant {
            PIECE_VALUES[Piece::WP.index()] - threshold
        } else {
            let cap = self.piece_at(dest);
            let mut value = if cap == Piece::Empty {
                0
            } else {
                PIECE_VALUES[cap.index()]
            };
            if mt.is_promotion() {
                next_piece = Some(mt.get_promotion(self.side).index());
                unsafe {
                    // There is a promotion
                    value += PIECE_VALUES[next_piece.unwrap_unchecked()]
                        - PIECE_VALUES[Piece::WP.index()];
                }
            }
            value - threshold
        };

        if score < 0 {
            return false;
        }

        score -= PIECE_VALUES[next_piece.unwrap_or(self.piece_at(src).index())];

        if score >= 0 {
            return true;
        }

        let mut occ = (self.sides[Colour::White as usize] | self.sides[Colour::Black as usize])
            ^ src.to_board()
            ^ dest.to_board();
        if mt == MoveKind::EnPassant {
            let ep_dest = self.en_passant.unwrap().shift::<8>(!self.side);
            occ ^= ep_dest.to_board();
        }

        let idx = dest.index();
        let mut attackers = ((KNIGHT_ATTACKS[idx] & self.pieces[Piece::WN.index()])
            | (KING_ATTACKS[idx] & self.pieces[Piece::WK.index()])
            | (PAWN_ATTACKS[Colour::White as usize][idx] & self.pieces[Piece::WP.index()])
            | (PAWN_ATTACKS[Colour::Black as usize][idx] & self.pieces[Piece::WP.index()])
            | (rook_attacks(occ.0, idx)
                & (self.pieces[Piece::WR.index()] | self.pieces[Piece::WQ.index()]))
            | (bishop_attacks(occ.0, idx)
                & (self.pieces[Piece::WB.index()] | self.pieces[Piece::WQ.index()])))
            & occ;

        let mut stm = !self.side;
        let diagonal = self.pieces[Piece::WB.index()] | self.pieces[Piece::WQ.index()];
        let normal = self.pieces[Piece::WR.index()] | self.pieces[Piece::WQ.index()];

        loop {
            let own_attackers = attackers & self.sides[stm as usize];
            if own_attackers == BitBoard::EMPTY {
                break;
            }

            let side_bb = self.sides[stm as usize];

            let att_sq_piece = Piece::COLOUR_PIECES[stm as usize]
                .iter()
                .find_map(|&piece| {
                    let squares = own_attackers & self.pieces[piece.index()] & side_bb;
                    if squares != BitBoard::EMPTY {
                        Some((squares.lsb(), piece))
                    } else {
                        None
                    }
                });
            let (att_sq, att) = att_sq_piece.unwrap();
            occ = occ.pop_bit(att_sq);

            if att.is_pawn() || att.is_bishop() || att.is_queen() {
                attackers |= bishop_attacks(occ.0, dest.index()) & diagonal;
            }

            if att.is_rook() || att.is_queen() {
                attackers |= rook_attacks(occ.0, dest.index()) & normal;
            }

            attackers &= occ;
            stm = !stm;

            score = -score - 1 - PIECE_VALUES[att.index()];
            if score >= 0 {
                if att.is_king() && attackers & self.sides[stm as usize] != BitBoard::EMPTY {
                    return self.side == stm;
                }

                break;
            }
        }

        self.side != stm
    }

    pub fn from_fen(state: &str) -> Self {
        let fen: Vec<&str> = state.split_whitespace().take(6).collect();

        if fen.len() != 6 {
            panic!("Invalid input FEN string");
        }

        let board_layout = fen[0];
        let mut board = Self::new();
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
        board.calculate_threats();
        board.pinned_and_checkers();

        board
    }
}

// For debugging

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\x1B[2J\x1B[1;1H")?;
        writeln!(f, "  a b c d e f g h")?;
        writeln!(f, " ┌────────────────┐")?;

        for row in (0..8).rev() {
            write!(f, "{}│", row + 1)?;
            for col in 0..8 {
                let square = Square::from_row_col(row, col);
                let piece = self.piece_map[square.index()];
                let bg = if (row + col) % 2 == 0 {
                    "\x1b[48;2;240;217;181m"
                } else {
                    "\x1b[48;2;181;136;99m"
                };

                if piece == Piece::Empty {
                    write!(f, "{bg}  \x1b[0m")?;
                } else {
                    let fg = match piece.colour() {
                        Colour::White => "\x1b[38;2;255;255;255m",
                        Colour::Black => "\x1b[38;2;0;0;0m",
                    };
                    write!(f, "{bg}{fg}{piece} \x1b[0m")?;
                }
            }
            writeln!(f, "│")?;
        }

        writeln!(f, " └────────────────┘")
    }
}
