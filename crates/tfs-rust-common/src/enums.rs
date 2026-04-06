use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Direction {
    North = 0,
    East = 1,
    South = 2,
    West = 3,
    SouthWest = 4,
    SouthEast = 5,
    NorthWest = 6,
    NorthEast = 7,
}

/// Order matches TFS `CombatType_t` (`combat.h`) 0..=11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CombatType {
    Physical = 0,
    Energy = 1,
    Earth = 2,
    Fire = 3,
    Undefined = 4,
    LifeDrain = 5,
    ManaDrain = 6,
    Healing = 7,
    Drown = 8,
    Ice = 9,
    Holy = 10,
    Death = 11,
}

/// Order matches TFS `ConditionType_t` (`condition.h`) 0..=24.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ConditionType {
    None = 0,
    Poison = 1,
    Fire = 2,
    Energy = 3,
    Bleeding = 4,
    Haste = 5,
    Paralyze = 6,
    Outfit = 7,
    Invisible = 8,
    Light = 9,
    ManaShield = 10,
    Infight = 11,
    Drunk = 12,
    ExhaustWeapon = 13,
    ExhaustCombat = 14,
    ExhaustHeal = 15,
    Muted = 16,
    ChannelMutedTicks = 17,
    YellTicks = 18,
    Attributes = 19,
    Freezing = 20,
    Dazzled = 21,
    Cursed = 22,
    ExhaustGroup = 23,
    Pz = 24,
}

/// Order matches TFS `Skulls_t` (`enums.h`) for protocol skull byte 0..=6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SkullType {
    None = 0,
    Yellow = 1,
    Green = 2,
    White = 3,
    Red = 4,
    Black = 5,
    Orange = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneType {
    Normal,
    /// Open PvP field (`TILESTATE_PVPZONE` in TFS).
    Pvp,
    Protection,
    NoPvp,
    NoLogout,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemGroup {
    None,
    Ground,
    Container,
    Weapon,
    Ammunition,
    Armor,
    Chargeable,
    Teleport,
    MagicField,
    Writeable,
    Key,
    Splash,
    Fluid,
    Door,
    Deprecated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaponType {
    None,
    Sword,
    Club,
    Axe,
    Shield,
    Distance,
    Wand,
    Ammunition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Skill {
    Fist = 0,
    Club = 1,
    Sword = 2,
    Axe = 3,
    Distance = 4,
    Shield = 5,
    Fishing = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerSex {
    Female = 0,
    Male = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldType {
    NoPvp,
    Pvp,
    PvpEnforced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SpeakType {
    Say = 1,
    Whisper = 2,
    Yell = 3,
    PrivatePlayerToNpc = 4,
    PrivateNpcToPlayer = 5,
    Private = 6,
    ChannelYellow = 7,
    ChannelWhite = 8,
    ChannelRed = 9,
    ChannelOrange = 10,
    PrivateRed = 11,
    MonsterSay = 36,
    MonsterYell = 37,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MagicEffect {
    DrawBlood = 0,
    LoseEnergy = 1,
    Poff = 2,
    BlockHit = 3,
    ExplosionArea = 4,
    ExplosionDamage = 5,
    FireArea = 6,
    FireDamage = 7,
    EnergyArea = 8,
    EnergyDamage = 9,
    Unknown = 10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShootEffect {
    Spear = 1,
    Bolt = 2,
    Arrow = 3,
    Fire = 4,
    Energy = 5,
    PoisonArrow = 6,
    BurstArrow = 7,
    Unknown = 8,
}

// --- `Display` for logging (`tasks/Idioms-audit.md` §9) ---

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::North => "north",
            Self::East => "east",
            Self::South => "south",
            Self::West => "west",
            Self::SouthWest => "south-west",
            Self::SouthEast => "south-east",
            Self::NorthWest => "north-west",
            Self::NorthEast => "north-east",
        })
    }
}

impl fmt::Display for CombatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Physical => "physical",
            Self::Energy => "energy",
            Self::Earth => "earth",
            Self::Fire => "fire",
            Self::Undefined => "undefined",
            Self::LifeDrain => "life-drain",
            Self::ManaDrain => "mana-drain",
            Self::Healing => "healing",
            Self::Drown => "drown",
            Self::Ice => "ice",
            Self::Holy => "holy",
            Self::Death => "death",
        })
    }
}

impl fmt::Display for ConditionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Poison => "poison",
            Self::Fire => "fire",
            Self::Energy => "energy",
            Self::Bleeding => "bleeding",
            Self::Haste => "haste",
            Self::Paralyze => "paralyze",
            Self::Outfit => "outfit",
            Self::Invisible => "invisible",
            Self::Light => "light",
            Self::ManaShield => "mana-shield",
            Self::Infight => "infight",
            Self::Drunk => "drunk",
            Self::ExhaustWeapon => "exhaust-weapon",
            Self::ExhaustCombat => "exhaust-combat",
            Self::ExhaustHeal => "exhaust-heal",
            Self::Muted => "muted",
            Self::ChannelMutedTicks => "channel-muted-ticks",
            Self::YellTicks => "yell-ticks",
            Self::Attributes => "attributes",
            Self::Freezing => "freezing",
            Self::Dazzled => "dazzled",
            Self::Cursed => "cursed",
            Self::ExhaustGroup => "exhaust-group",
            Self::Pz => "pz",
        })
    }
}

impl fmt::Display for SkullType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::White => "white",
            Self::Red => "red",
            Self::Black => "black",
            Self::Orange => "orange",
        })
    }
}

impl fmt::Display for ZoneType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Normal => "normal",
            Self::Pvp => "pvp",
            Self::Protection => "protection",
            Self::NoPvp => "no-pvp",
            Self::NoLogout => "no-logout",
        })
    }
}

impl fmt::Display for ItemGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Ground => "ground",
            Self::Container => "container",
            Self::Weapon => "weapon",
            Self::Ammunition => "ammunition",
            Self::Armor => "armor",
            Self::Chargeable => "chargeable",
            Self::Teleport => "teleport",
            Self::MagicField => "magic-field",
            Self::Writeable => "writeable",
            Self::Key => "key",
            Self::Splash => "splash",
            Self::Fluid => "fluid",
            Self::Door => "door",
            Self::Deprecated => "deprecated",
        })
    }
}

impl fmt::Display for WeaponType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Sword => "sword",
            Self::Club => "club",
            Self::Axe => "axe",
            Self::Shield => "shield",
            Self::Distance => "distance",
            Self::Wand => "wand",
            Self::Ammunition => "ammunition",
        })
    }
}

impl fmt::Display for Skill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Fist => "fist",
            Self::Club => "club",
            Self::Sword => "sword",
            Self::Axe => "axe",
            Self::Distance => "distance",
            Self::Shield => "shield",
            Self::Fishing => "fishing",
        })
    }
}

impl fmt::Display for PlayerSex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Female => "female",
            Self::Male => "male",
        })
    }
}

impl fmt::Display for WorldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoPvp => "no-pvp",
            Self::Pvp => "pvp",
            Self::PvpEnforced => "pvp-enforced",
        })
    }
}

impl fmt::Display for SpeakType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Say => "say",
            Self::Whisper => "whisper",
            Self::Yell => "yell",
            Self::PrivatePlayerToNpc => "private-player-to-npc",
            Self::PrivateNpcToPlayer => "private-npc-to-player",
            Self::Private => "private",
            Self::ChannelYellow => "channel-yellow",
            Self::ChannelWhite => "channel-white",
            Self::ChannelRed => "channel-red",
            Self::ChannelOrange => "channel-orange",
            Self::PrivateRed => "private-red",
            Self::MonsterSay => "monster-say",
            Self::MonsterYell => "monster-yell",
        })
    }
}

impl fmt::Display for MagicEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DrawBlood => "draw-blood",
            Self::LoseEnergy => "lose-energy",
            Self::Poff => "poff",
            Self::BlockHit => "block-hit",
            Self::ExplosionArea => "explosion-area",
            Self::ExplosionDamage => "explosion-damage",
            Self::FireArea => "fire-area",
            Self::FireDamage => "fire-damage",
            Self::EnergyArea => "energy-area",
            Self::EnergyDamage => "energy-damage",
            Self::Unknown => "unknown",
        })
    }
}

impl fmt::Display for ShootEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Spear => "spear",
            Self::Bolt => "bolt",
            Self::Arrow => "arrow",
            Self::Fire => "fire",
            Self::Energy => "energy",
            Self::PoisonArrow => "poison-arrow",
            Self::BurstArrow => "burst-arrow",
            Self::Unknown => "unknown",
        })
    }
}

// --- Wire / TFS `u8` conversions (see `tasks/Idioms-audit.md` §6c) ---

impl TryFrom<u8> for Direction {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::North),
            1 => Ok(Self::East),
            2 => Ok(Self::South),
            3 => Ok(Self::West),
            4 => Ok(Self::SouthWest),
            5 => Ok(Self::SouthEast),
            6 => Ok(Self::NorthWest),
            7 => Ok(Self::NorthEast),
            _ => Err(()),
        }
    }
}

impl TryFrom<u8> for CombatType {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        Ok(match v {
            0 => Self::Physical,
            1 => Self::Energy,
            2 => Self::Earth,
            3 => Self::Fire,
            4 => Self::Undefined,
            5 => Self::LifeDrain,
            6 => Self::ManaDrain,
            7 => Self::Healing,
            8 => Self::Drown,
            9 => Self::Ice,
            10 => Self::Holy,
            11 => Self::Death,
            _ => return Err(()),
        })
    }
}

impl TryFrom<u8> for ConditionType {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        Ok(match v {
            0 => Self::None,
            1 => Self::Poison,
            2 => Self::Fire,
            3 => Self::Energy,
            4 => Self::Bleeding,
            5 => Self::Haste,
            6 => Self::Paralyze,
            7 => Self::Outfit,
            8 => Self::Invisible,
            9 => Self::Light,
            10 => Self::ManaShield,
            11 => Self::Infight,
            12 => Self::Drunk,
            13 => Self::ExhaustWeapon,
            14 => Self::ExhaustCombat,
            15 => Self::ExhaustHeal,
            16 => Self::Muted,
            17 => Self::ChannelMutedTicks,
            18 => Self::YellTicks,
            19 => Self::Attributes,
            20 => Self::Freezing,
            21 => Self::Dazzled,
            22 => Self::Cursed,
            23 => Self::ExhaustGroup,
            24 => Self::Pz,
            _ => return Err(()),
        })
    }
}

impl TryFrom<u8> for SkullType {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        Ok(match v {
            0 => Self::None,
            1 => Self::Yellow,
            2 => Self::Green,
            3 => Self::White,
            4 => Self::Red,
            5 => Self::Black,
            6 => Self::Orange,
            _ => return Err(()),
        })
    }
}

impl TryFrom<u8> for SpeakType {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::Say),
            2 => Ok(Self::Whisper),
            3 => Ok(Self::Yell),
            4 => Ok(Self::PrivatePlayerToNpc),
            5 => Ok(Self::PrivateNpcToPlayer),
            6 => Ok(Self::Private),
            7 => Ok(Self::ChannelYellow),
            8 => Ok(Self::ChannelWhite),
            9 => Ok(Self::ChannelRed),
            10 => Ok(Self::ChannelOrange),
            11 => Ok(Self::PrivateRed),
            36 => Ok(Self::MonsterSay),
            37 => Ok(Self::MonsterYell),
            _ => Err(()),
        }
    }
}

impl TryFrom<u8> for MagicEffect {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        Ok(match v {
            0 => Self::DrawBlood,
            1 => Self::LoseEnergy,
            2 => Self::Poff,
            3 => Self::BlockHit,
            4 => Self::ExplosionArea,
            5 => Self::ExplosionDamage,
            6 => Self::FireArea,
            7 => Self::FireDamage,
            8 => Self::EnergyArea,
            9 => Self::EnergyDamage,
            10 => Self::Unknown,
            _ => return Err(()),
        })
    }
}

impl TryFrom<u8> for ShootEffect {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        Ok(match v {
            1 => Self::Spear,
            2 => Self::Bolt,
            3 => Self::Arrow,
            4 => Self::Fire,
            5 => Self::Energy,
            6 => Self::PoisonArrow,
            7 => Self::BurstArrow,
            8 => Self::Unknown,
            _ => return Err(()),
        })
    }
}
