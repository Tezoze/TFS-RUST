//! Blessing names shared by the 772 quest flags and the later bitfield.
//!
//! Corpus: `TPlayer::Death` counts `GetQuestValue(101..105)` (`crplayer.cc:343-348`).
//! Announcer names: `moveuse.cc:1914-1936`. Quest 199 is Kawill's gate, not a blessing.
//! Later codecs store the same five, plus twist, as bits 0–5.

/// One blessing. Bit order is shielding, embrace, suns, spark, solitude, twist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blessing {
    Shielding,
    Embrace,
    Suns,
    Spark,
    Solitude,
    Twist,
}

impl Blessing {
    pub const ALL: [Blessing; 6] = [
        Blessing::Shielding,
        Blessing::Embrace,
        Blessing::Suns,
        Blessing::Spark,
        Blessing::Solitude,
        Blessing::Twist,
    ];

    /// The five blessings death loss counts. Twist is not one of them.
    pub const COUNTED: [Blessing; 5] = [
        Blessing::Shielding,
        Blessing::Embrace,
        Blessing::Suns,
        Blessing::Spark,
        Blessing::Solitude,
    ];

    pub fn word(self) -> &'static str {
        match self {
            Blessing::Shielding => "shielding",
            Blessing::Embrace => "embrace",
            Blessing::Suns => "suns",
            Blessing::Spark => "spark",
            Blessing::Solitude => "solitude",
            Blessing::Twist => "twist",
        }
    }

    /// 1-based index. `1` is shielding.
    pub fn index(self) -> i32 {
        self.bit() as i32 + 1
    }

    pub fn bit(self) -> u32 {
        match self {
            Blessing::Shielding => 0,
            Blessing::Embrace => 1,
            Blessing::Suns => 2,
            Blessing::Spark => 3,
            Blessing::Solitude => 4,
            Blessing::Twist => 5,
        }
    }

    pub fn mask(self) -> i8 {
        1i8.wrapping_shl(self.bit())
    }

    /// 772 quest id. Twist has none. Quest 199 is not a blessing.
    pub fn quest_id(self) -> Option<u32> {
        match self {
            Blessing::Solitude => Some(101),
            Blessing::Spark => Some(102),
            Blessing::Suns => Some(103),
            Blessing::Shielding => Some(104),
            Blessing::Embrace => Some(105),
            Blessing::Twist => None,
        }
    }

    pub fn from_word(word: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|blessing| blessing.word().eq_ignore_ascii_case(word))
    }

    pub fn from_index(index: i32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|blessing| blessing.index() == index)
    }

    pub fn from_quest(storage_id: u32) -> Option<Self> {
        Self::COUNTED
            .into_iter()
            .find(|blessing| blessing.quest_id() == Some(storage_id))
    }
}
