use crate::game::bitboard::BitBoard;

use super::piece::Colour;

/// Represents a square on a chessboard using a 0-63 index (a1 = 0, h8 = 63).
///
/// The square is stored as a `u8` where the least significant bit (LSB) corresponds to a1.
/// This struct provides methods for converting between algebraic notation (e.g., "e4"),
/// row-column coordinates, and bitboard representations.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug)]
pub struct Square(u8);

impl Square {
    /// Total number of squares on a chessboard.
    pub const COUNT: usize = 64;

    /// Creates a new square from an algebraic notation string (e.g., "e4").
    ///
    /// In debug mode, panics if the position string is not a valid chess square (e.g., "z9" or not exactly 2 characters).
    /// In release mode, assumes the input is valid and does not perform checks for performance.
    pub const fn from(pos: &str) -> Self {
        #[cfg(debug_assertions)]
        {
            if pos.len() != 2 {
                panic!("Invalid algebraic notation: length must be 2");
            }
        }

        let bytes = pos.as_bytes();
        let col = bytes[0].wrapping_sub(b'a');
        let row = bytes[1].wrapping_sub(b'1');

        #[cfg(debug_assertions)]
        {
            if col > 7 || row > 7 {
                panic!("Invalid algebraic notation: out of bounds");
            }
        }

        Self::new(row * 8 + col)
    }

    /// Creates a new square from a 0-63 index.
    pub const fn new(index: u8) -> Self {
        Self(index)
    }

    /// Returns the index of the square (0-63).
    pub const fn index(&self) -> usize {
        self.0 as usize
    }

    /// Creates a square from row (0-7) and column (0-7) coordinates.
    pub const fn from_row_col(row: usize, col: usize) -> Self {
        Self((row * 8 + col) as u8)
    }

    /// Returns the row (rank) of the square (0-7, where 0 is rank 1).
    pub const fn row(&self) -> usize {
        self.0 as usize / 8
    }

    /// Returns the column (file) of the square (0-7, where 0 is file a).
    pub const fn col(&self) -> usize {
        self.0 as usize % 8
    }

    /// Converts the square to a `BitBoard` with only this square set.
    pub const fn to_board(self) -> BitBoard {
        BitBoard(1 << self.0)
    }

    pub fn shift<const AMOUNT: u8>(self, side: Colour) -> Self {
        let sign = 1u8 - (side as u8 * 2); // White = +1, Black = -1 (mod 256)
        Square::new(self.0 + AMOUNT * sign)
    }

    /// Attempts to move the square by the given file and rank deltas.
    ///
    /// Returns `None` if the resulting position is off the board.
    /// With LSB = a1, positive `file_delta` moves up (e.g., a2 to a3),
    /// and positive `rank_delta` moves right (e.g., a2 to b2).
    /// Only < 8 is checked as converting i8 negative numbers results in >8
    pub fn jump_check(self, rank_delta: i8, file_delta: i8) -> Option<Self> {
        let file = ((self.0 % 8) as i8 + rank_delta) as u8;
        let rank = ((self.0 / 8) as i8 + file_delta) as u8;
        if (file | rank) < 8 {
            Some(Self(rank * 8 + file))
        } else {
            None
        }
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let file = (self.col() as u8 + b'a') as char;
        let rank = (self.row() + 1).to_string();
        write!(f, "{file}{rank}")
    }
}
