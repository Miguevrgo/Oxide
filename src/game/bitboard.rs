use crate::game::square::Square;

/// A 64-bit representation of a chess board, where each bit corresponds to a square.
/// A `1` indicates the presence of a piece, and a `0` indicates an empty square.
///
/// This struct is designed for efficient manipulation of chess positions using
/// bitwise operations, designed so that the LSB correponds to A1 and MSB to H8
#[derive(PartialEq, Eq, PartialOrd, Clone, Copy, Debug, Default, Hash)]
pub struct BitBoard(pub u64);

impl std::ops::BitAnd for BitBoard {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        BitBoard(self.0 & rhs.0)
    }
}

impl std::ops::BitOr for BitBoard {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        BitBoard(self.0 | rhs.0)
    }
}

impl std::ops::BitXor for BitBoard {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        BitBoard(self.0 ^ rhs.0)
    }
}

impl std::ops::Shl<u8> for BitBoard {
    type Output = Self;
    fn shl(self, rhs: u8) -> Self {
        BitBoard(self.0 << rhs)
    }
}

impl std::ops::Shr<u8> for BitBoard {
    type Output = Self;
    fn shr(self, rhs: u8) -> Self {
        BitBoard(self.0 >> rhs)
    }
}

impl std::ops::Not for BitBoard {
    type Output = Self;
    fn not(self) -> Self {
        BitBoard(!self.0)
    }
}

impl std::ops::BitAndAssign for BitBoard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl std::ops::BitOrAssign for BitBoard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitXorAssign for BitBoard {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl BitBoard {
    /// An empty bitboard with no pieces (all bits set to 0).
    pub const EMPTY: Self = Self(0);

    /// Starting ranks for pawns: White (rank 2) and Black (rank 7).
    pub const START_RANKS: [Self; 2] = [Self(0x000000000000FF00), Self(0x00FF000000000000)];
    /// Ranks where promotion can occur: White (rank 7) and Black (rank 2).
    pub const PROMO_RANKS: [Self; 2] = [Self(0xFF00000000000000), Self(0x00000000000000FF)];
    /// Starting positions for both kings
    pub const KING_START_POS: Self = Self(0x1000000000000010);

    /// Intermediate squares for white king-side castle
    pub const WHITE_KING_CASTLE: Self = Self(0x0000000000000060);
    /// Intermediate squares for black king-side castle
    pub const BLACK_KING_CASTLE: Self = Self(0x6000000000000000);
    /// Intermediate squares for white queen-side castle
    pub const WHITE_QUEEN_CASTLE: Self = Self(0x000000000000000e);
    /// Intermediate squares for black queen-side castle
    pub const BLACK_QUEEN_CASTLE: Self = Self(0x0e00000000000000);

    // White squares
    pub const WHITE_SQUARES: Self = Self(0x55AA55AA55AA55AA);
    // Black squares
    pub const BLACK_SQUARES: Self = Self(0xAA55AA55AA55AA55);

    /// Checks if a specific square contains a piece.
    ///
    /// # Arguments
    ///
    /// * `square` - The square to check.
    ///
    /// # Returns
    ///
    /// `true` if the square is occupied, `false` otherwise.
    pub fn get_bit(self, square: Square) -> bool {
        self.0 & (1u64 << square.index()) != 0
    }

    /// Sets the bit at the given square, indicating a piece is present.
    ///
    ///
    /// # Arguments
    ///
    /// * `square` - The square where the bit will be set.
    ///
    /// # Returns
    ///
    /// A new `BitBoard` with the specified bit set.
    pub fn set_bit(self, square: Square) -> Self {
        Self(self.0 | (1u64 << square.index()))
    }

    /// Clears the bit at the given square, indicating the piece is removed.
    ///
    /// # Arguments
    ///
    /// * `square` - The square where the bit will be cleared.
    ///
    /// # Returns
    ///
    /// A new `BitBoard` with the specified bit cleared.
    pub fn pop_bit(self, square: Square) -> Self {
        Self(self.0 & !(1u64 << square.index()))
    }

    /// Counts the number of occupied squares in the bitboard.
    ///
    /// # Returns
    ///
    /// The number of bits set to 1.
    pub fn count_bits(self) -> u32 {
        self.0.count_ones()
    }

    /// Returns the square corresponding to the least significant bit (LSB).
    ///
    /// # Returns
    ///
    /// The `Square` of the rightmost set bit.
    ///
    /// # Panics
    ///
    /// Panics if the bitboard is empty (no bits set).
    pub fn lsb(self) -> Square {
        Square::new(self.0.trailing_zeros() as usize)
    }
}

impl std::fmt::Display for BitBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  a b c d e f g h")?;
        writeln!(f, " ┌────────────────┐")?;
        for rank in (0..8).rev() {
            write!(f, "{}│", rank + 1)?;
            for file in 0..8 {
                let index = rank * 8 + file;
                let bit = (self.0 >> index) & 1;
                write!(f, "{} ", if bit == 1 { "1" } else { "0" })?;
            }
            writeln!(f, "│")?;
        }
        writeln!(f, " └────────────────┘")
    }
}
