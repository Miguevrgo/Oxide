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
        if rights == "-" {
            return Self::NONE;
        }

        let mut right = Self::NONE;
        for token in rights.chars() {
            right.0 |= match token {
                'K' => Self::WK,
                'Q' => Self::WQ,
                'k' => Self::BK,
                'q' => Self::BQ,
                _ => panic!("Invalid CastlingRights in FEN"),
            };
        }

        right
    }
}

pub const CASTLE_MASK: [u8; 64] = {
    let mut m = [0xFF_u8; 64];
    m[0] = !CastlingRights::WQ;
    m[4] = !(CastlingRights::WK | CastlingRights::WQ);
    m[7] = !CastlingRights::WK;
    m[56] = !CastlingRights::BQ;
    m[60] = !(CastlingRights::BK | CastlingRights::BQ);
    m[63] = !CastlingRights::BK;
    m
};
