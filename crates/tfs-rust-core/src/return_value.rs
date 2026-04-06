//! Return values for operations (queryAdd, queryRemove, etc.)
// C++ reference: `src/enums.h:389-465`

/// Return value for operations, matching C++ ReturnValue enum
// C++ ref: `src/enums.h:389-465`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReturnValue {
    NoError = 0,
    NotPossible,
    NotEnoughRoom,
    PlayerIsPzLocked,
    PlayerIsNotInvited,
    CannotThrow,
    ThereIsNoWay,
    DestinationOutOfReach,
    CreatureBlock,
    NotMoveable,
    DropTwoHandedItem,
    BothHandsNeedToBeFree,
    CanOnlyUseOneWeapon,
    NeedExchange,
    CannotBeDressed,
    PutThisObjectInYourHand,
    PutThisObjectInBothHands,
    TooFarAway,
    FirstGoDownStairs,
    FirstGoUpStairs,
    ContainerNotEnoughRoom,
    NotEnoughCapacity,
    CannotPickup,
    ThisIsImpossible,
    DepotIsFull,
    CreatureDoesNotExist,
    CannotUseThisObject,
    PlayerWithThisNameIsNotOnline,
    NotRequiredLevelToUseRune,
    YouAreAlreadyTrading,
    ThisPlayerIsAlreadyTrading,
    YouMayNotLogoutDuringAFight,
    DirectPlayerShoot,
    NotEnoughLevel,
    NotEnoughMagicLevel,
    NotEnoughMana,
    NotEnoughSoul,
    YouAreExhausted,
    YouCannotUseObjectsThatFast,
    PlayerIsNotReachable,
    CanOnlyUseThisRuneOnCreatures,
    ActionNotPermittedInProtectionZone,
    YouMayNotAttackThisPlayer,
    YouMayNotAttackAPersonInProtectionZone,
    YouMayNotAttackAPersonWhileInProtectionZone,
    YouMayNotAttackThisCreature,
    YouCanOnlyUseItOnCreatures,
    CreatureIsNotReachable,
    TurnSecureModeToAttackUnmarkedPlayers,
    YouNeedPremiumAccount,
    YouNeedToLearnThisSpell,
    YourVocationCannotUseThisSpell,
    YouNeedAWeaponToUseThisSpell,
    PlayerIsPzLockedLeavePvpZone,
    PlayerIsPzLockedEnterPvpZone,
    ActionNotPermittedInANoPvpZone,
    YouCannotLogoutHere,
    YouNeedAMagicItemToCastSpell,
    CannotConjureItemHere,
    YouNeedToSplitYourSpears,
    NameIsTooAmbiguous,
    CanOnlyUseOneShield,
    NoPartyMembersInRange,
    YouAreNotTheOwner,
    NoSuchRaidExists,
    AnotherRaidIsAlreadyExecuting,
    TradePlayerFarAway,
    YouDontOwnThisHouse,
    TradePlayerAlreadyOwnsAHouse,
    TradePlayerHighestBidder,
    YouCannotTradeThisHouse,
    YouDontHaveRequiredProfession,
    CannotMoveItemIsNotStoreItem,
    ItemCannotBeMovedThere,
    YouCannotUseThisBed,
}

impl ReturnValue {
    /// Check if the return value indicates success
    pub fn is_success(self) -> bool {
        matches!(self, ReturnValue::NoError)
    }

    /// Check if the return value indicates failure
    pub fn is_error(self) -> bool {
        !self.is_success()
    }

    /// Get a human-readable description of the return value
    pub fn description(self) -> &'static str {
        match self {
            ReturnValue::NoError => "No error.",
            ReturnValue::NotPossible => "Sorry, not possible.",
            ReturnValue::NotEnoughRoom => "There is not enough room.",
            ReturnValue::PlayerIsPzLocked => "You may not enter a protection zone after attacking another player.",
            ReturnValue::PlayerIsNotInvited => "You are not invited.",
            ReturnValue::CannotThrow => "You cannot throw there.",
            ReturnValue::ThereIsNoWay => "There is no way.",
            ReturnValue::DestinationOutOfReach => "Destination is out of range.",
            ReturnValue::CreatureBlock => "You cannot throw there.",
            ReturnValue::NotMoveable => "You cannot move this object.",
            ReturnValue::DropTwoHandedItem => "Drop the weapon first.",
            ReturnValue::BothHandsNeedToBeFree => "Both hands need to be free.",
            ReturnValue::CanOnlyUseOneWeapon => "You may only use one weapon.",
            ReturnValue::NeedExchange => "You need to exchange items.",
            ReturnValue::CannotBeDressed => "This cannot be dressed.",
            ReturnValue::PutThisObjectInYourHand => "Put this object in your hand.",
            ReturnValue::PutThisObjectInBothHands => "Put this object in both hands.",
            ReturnValue::TooFarAway => "Too far away.",
            ReturnValue::FirstGoDownStairs => "First go downstairs.",
            ReturnValue::FirstGoUpStairs => "First go upstairs.",
            ReturnValue::ContainerNotEnoughRoom => "You cannot put more objects in this container.",
            ReturnValue::NotEnoughCapacity => "This object is too heavy for you to carry.",
            ReturnValue::CannotPickup => "You cannot take this object.",
            ReturnValue::ThisIsImpossible => "This is impossible.",
            ReturnValue::DepotIsFull => "You cannot put more than 2000 items in a depot.",
            ReturnValue::CreatureDoesNotExist => "Creature does not exist.",
            ReturnValue::CannotUseThisObject => "You cannot use this object.",
            ReturnValue::PlayerWithThisNameIsNotOnline => "A player with this name is not online.",
            ReturnValue::NotRequiredLevelToUseRune => "You do not have the required magic level to use this rune.",
            ReturnValue::YouAreAlreadyTrading => "You are already trading.",
            ReturnValue::ThisPlayerIsAlreadyTrading => "This player is already trading.",
            ReturnValue::YouMayNotLogoutDuringAFight => "You may not logout during or immediately after a fight.",
            ReturnValue::DirectPlayerShoot => "You cannot use this item on yourself.",
            ReturnValue::NotEnoughLevel => "You do not have enough level.",
            ReturnValue::NotEnoughMagicLevel => "You do not have enough magic level.",
            ReturnValue::NotEnoughMana => "You do not have enough mana.",
            ReturnValue::NotEnoughSoul => "You do not have enough soul.",
            ReturnValue::YouAreExhausted => "You are exhausted.",
            ReturnValue::YouCannotUseObjectsThatFast => "You cannot use objects that fast.",
            ReturnValue::PlayerIsNotReachable => "Player is not reachable.",
            ReturnValue::CanOnlyUseThisRuneOnCreatures => "You can only use this rune on creatures.",
            ReturnValue::ActionNotPermittedInProtectionZone => "This action is not permitted in a protection zone.",
            ReturnValue::YouMayNotAttackThisPlayer => "You may not attack this player.",
            ReturnValue::YouMayNotAttackAPersonInProtectionZone => "You may not attack a person in a protection zone.",
            ReturnValue::YouMayNotAttackAPersonWhileInProtectionZone => "You may not attack a person while you are in a protection zone.",
            ReturnValue::YouMayNotAttackThisCreature => "You may not attack this creature.",
            ReturnValue::YouCanOnlyUseItOnCreatures => "You can only use it on creatures.",
            ReturnValue::CreatureIsNotReachable => "Creature is not reachable.",
            ReturnValue::TurnSecureModeToAttackUnmarkedPlayers => "Turn secure mode off if you really want to attack unmarked players.",
            ReturnValue::YouNeedPremiumAccount => "You need a premium account.",
            ReturnValue::YouNeedToLearnThisSpell => "You need to learn this spell first.",
            ReturnValue::YourVocationCannotUseThisSpell => "Your vocation cannot use this spell.",
            ReturnValue::YouNeedAWeaponToUseThisSpell => "You need to equip a weapon to use this spell.",
            ReturnValue::PlayerIsPzLockedLeavePvpZone => "You may not leave a protection zone after attacking another player.",
            ReturnValue::PlayerIsPzLockedEnterPvpZone => "You may not enter a protection zone after attacking another player.",
            ReturnValue::ActionNotPermittedInANoPvpZone => "This action is not permitted in a no PvP zone.",
            ReturnValue::YouCannotLogoutHere => "You may not logout here.",
            ReturnValue::YouNeedAMagicItemToCastSpell => "You need a magic item to cast this spell.",
            ReturnValue::CannotConjureItemHere => "You cannot conjure items here.",
            ReturnValue::YouNeedToSplitYourSpears => "You must split your spears first.",
            ReturnValue::NameIsTooAmbiguous => "This name is too ambiguous.",
            ReturnValue::CanOnlyUseOneShield => "You may use only one shield.",
            ReturnValue::NoPartyMembersInRange => "No party members in range.",
            ReturnValue::YouAreNotTheOwner => "You are not the owner.",
            ReturnValue::NoSuchRaidExists => "No such raid exists.",
            ReturnValue::AnotherRaidIsAlreadyExecuting => "Another raid is already executing.",
            ReturnValue::TradePlayerFarAway => "Trade player is too far away.",
            ReturnValue::YouDontOwnThisHouse => "You don't own this house.",
            ReturnValue::TradePlayerAlreadyOwnsAHouse => "Trade player already owns a house.",
            ReturnValue::TradePlayerHighestBidder => "Trade player is the highest bidder.",
            ReturnValue::YouCannotTradeThisHouse => "You cannot trade this house.",
            ReturnValue::YouDontHaveRequiredProfession => "You don't have the required profession.",
            ReturnValue::CannotMoveItemIsNotStoreItem => "You cannot move this item.",
            ReturnValue::ItemCannotBeMovedThere => "This item cannot be moved there.",
            ReturnValue::YouCannotUseThisBed => "You cannot use this bed.",
        }
    }
}

impl std::fmt::Display for ReturnValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_return_value_success() {
        assert!(ReturnValue::NoError.is_success());
        assert!(!ReturnValue::NotPossible.is_success());
    }

    #[test]
    fn test_return_value_display() {
        let rv = ReturnValue::NotEnoughCapacity;
        assert_eq!(rv.to_string(), "This object is too heavy for you to carry.");
    }
}
