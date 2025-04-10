/// Castling rights struct
/// Implemented through a flag bit vector. This allows for fast castle update without needing
/// bitboard lookups.
///
///  WK | WQ | BK | BQ  --> only using least significant 8 bits
///  08   04   02   01
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Debug, Hash)]
pub struct CastlingRights(pub u8);

impl CastlingRights {
    pub const WK: u8 = 0x08;
    pub const WQ: u8 = 0x04;
    pub const BK: u8 = 0x02;
    pub const BQ: u8 = 0x01;
    pub const NONE: CastlingRights = CastlingRights(0);

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub fn from(rights: &str) -> Self {
        let mut right = Self::NONE;

        if rights == "-" {
            return right;
        }

        for token in rights.chars() {
            match token {
                'K' => {
                    right.0 |= Self::WK;
                }
                'Q' => {
                    right.0 |= Self::WQ;
                }
                'k' => {
                    right.0 |= Self::BK;
                }
                'q' => {
                    right.0 |= Self::BQ;
                }
                _ => panic!("Invalid CastlingRights in FEN"),
            }
        }

        right
    }
}
