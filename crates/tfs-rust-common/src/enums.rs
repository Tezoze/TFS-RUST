#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombatType {
    Physical,
    Energy,
    Earth,
    Fire,
    Undefined,
    LifeDrain,
    ManaDrain,
    Healing,
    Drown,
    Ice,
    Holy,
    Death,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConditionType {
    None,
    Poison,
    Fire,
    Energy,
    Bleeding,
    Haste,
    Paralyze,
    Outfit,
    Invisible,
    Light,
    ManaShield,
    Infight,
    Drunk,
    ExhaustWeapon,
    ExhaustCombat,
    ExhaustHeal,
    Muted,
    ChannelMutedTicks,
    YellTicks,
    Attributes,
    Freezing,
    Dazzled,
    Cursed,
    ExhaustGroup,
    Pz,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkullType {
    None,
    Yellow,
    Green,
    White,
    Red,
    Black,
    Orange,
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
pub enum ReturnValue {
    NoError,
    NotPossible,
    NotEnoughRoom,
    PlayerIsPzLocked,
    PlayerIsNotInvited,
    CannotThrow,
    ThereIsNoWay,
    CreatureBlocksPath,
    PlayerIsNotPremium,
    PlayerIsMuted,
    // (Truncated for brevity, normally ~50 variants from TFS RET_ constants)
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
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShootEffect {
    Spear = 1,
    Bolt = 2,
    Arrow = 3,
    Fire = 4,
    Energy = 5,
    PoisonArrow = 6,
    BurstArrow = 7,
    Unknown,
}
