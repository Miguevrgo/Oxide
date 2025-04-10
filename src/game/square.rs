use crate::game::bitboard::BitBoard;

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

    /// Array of algebraic notations for all squares, ordered from a1 to h8.
    #[rustfmt::skip]
    #[allow(dead_code)]
    const STR: [&str; Self::COUNT] = [
        "a1", "b1", "c1", "d1", "e1", "f1", "g1", "h1",
        "a2", "b2", "c2", "d2", "e2", "f2", "g2", "h2",
        "a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3",
        "a4", "b4", "c4", "d4", "e4", "f4", "g4", "h4",
        "a5", "b5", "c5", "d5", "e5", "f5", "g5", "h5",
        "a6", "b6", "c6", "d6", "e6", "f6", "g6", "h6",
        "a7", "b7", "c7", "d7", "e7", "f7", "g7", "h7",
        "a8", "b8", "c8", "d8", "e8", "f8", "g8", "h8",
    ];

    /// Creates a new square from an algebraic notation string (e.g., "e4").
    ///
    /// In debug mode, panics if the position string is not a valid chess square (e.g., "z9" or not exactly 2 characters).
    /// In release mode, assumes the input is valid and does not perform checks for performance.
    pub fn from(pos: &str) -> Self {
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

        Self::new((row * 8 + col) as usize)
    }

    /// Creates a new square from a 0-63 index.
    pub const fn new(index: usize) -> Self {
        Self(index as u8)
    }

    /// Returns the index of the square (0-63).
    pub fn index(&self) -> usize {
        self.0 as usize
    }

    /// Creates a square from row (0-7) and column (0-7) coordinates.
    pub fn from_row_col(row: usize, col: usize) -> Self {
        Self((row * 8 + col) as u8)
    }

    /// Returns the row (rank) of the square (0-7, where 0 is rank 1).
    pub fn row(&self) -> usize {
        self.0 as usize / 8
    }

    /// Returns the column (file) of the square (0-7, where 0 is file a).
    pub fn col(&self) -> usize {
        self.0 as usize % 8
    }

    /// Converts the square to a `BitBoard` with only this square set.
    pub fn to_board(self) -> BitBoard {
        BitBoard(1 << self.0)
    }

    /// Attempts to move the square by the given file and rank deltas.
    ///
    /// Returns `None` if the resulting position is off the board.
    /// With LSB = a1, positive `file_delta` moves up (e.g., a2 to a3),
    /// and positive `rank_delta` moves right (e.g., a2 to b2).
    pub fn jump(self, rank_delta: i8, file_delta: i8) -> Option<Self> {
        let file = (self.0 % 8) as i8 + rank_delta;
        let rank = (self.0 / 8) as i8 + file_delta;
        if (0..8).contains(&file) && (0..8).contains(&rank) {
            Some(Self((rank * 8 + file) as u8))
        } else {
            None
        }
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let file = (self.col() as u8 + b'a') as char;
        let rank = (self.row() + 1).to_string();
        write!(f, "{}{}", file, rank)
    }
}
