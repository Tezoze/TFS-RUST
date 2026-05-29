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

    /// Player-visible text — C++ `getReturnMessage` (`src/tools.cpp` ~1015–1234).
    pub fn description(self) -> &'static str {
        match self {
            ReturnValue::NoError => "No error.",
            ReturnValue::DestinationOutOfReach => "Destination is out of range.",
            ReturnValue::NotMoveable => "You cannot move this object.",
            ReturnValue::DropTwoHandedItem => "Drop the double-handed object first.",
            ReturnValue::BothHandsNeedToBeFree => "Both hands need to be free.",
            ReturnValue::CannotBeDressed => "You cannot dress this object there.",
            ReturnValue::PutThisObjectInYourHand => "Put this object in your hand.",
            ReturnValue::PutThisObjectInBothHands => "Put this object in both hands.",
            ReturnValue::CanOnlyUseOneWeapon => "You may only use one weapon.",
            ReturnValue::TooFarAway => "You are too far away.",
            ReturnValue::FirstGoDownStairs => "First go downstairs.",
            ReturnValue::FirstGoUpStairs => "First go upstairs.",
            ReturnValue::NotEnoughCapacity => "This object is too heavy for you to carry.",
            ReturnValue::ContainerNotEnoughRoom => {
                "You cannot put more objects in this container."
            }
            ReturnValue::NeedExchange | ReturnValue::NotEnoughRoom => {
                "There is not enough room."
            }
            ReturnValue::CannotPickup => "You cannot take this object.",
            ReturnValue::CannotThrow => "You cannot throw there.",
            ReturnValue::ThereIsNoWay => "There is no way.",
            ReturnValue::ThisIsImpossible => "This is impossible.",
            ReturnValue::PlayerIsPzLocked => {
                "You can not enter a protection zone after attacking another player."
            }
            ReturnValue::PlayerIsNotInvited => "You are not invited.",
            ReturnValue::CreatureDoesNotExist => "Creature does not exist.",
            ReturnValue::DepotIsFull => "You cannot put more items in this depot.",
            ReturnValue::CannotUseThisObject => "You cannot use this object.",
            ReturnValue::PlayerWithThisNameIsNotOnline => {
                "A player with this name is not online."
            }
            ReturnValue::NotRequiredLevelToUseRune => {
                "You do not have the required magic level to use this rune."
            }
            ReturnValue::YouAreAlreadyTrading => {
                "You are already trading. Finish this trade first."
            }
            ReturnValue::ThisPlayerIsAlreadyTrading => "This player is already trading.",
            ReturnValue::YouMayNotLogoutDuringAFight => {
                "You may not logout during or immediately after a fight!"
            }
            ReturnValue::DirectPlayerShoot => {
                "You are not allowed to shoot directly on players."
            }
            ReturnValue::NotEnoughLevel => "Your level is too low.",
            ReturnValue::NotEnoughMagicLevel => "You do not have enough magic level.",
            ReturnValue::NotEnoughMana => "You do not have enough mana.",
            ReturnValue::NotEnoughSoul => "You do not have enough soul.",
            ReturnValue::YouAreExhausted => "You are exhausted.",
            ReturnValue::YouCannotUseObjectsThatFast => "You cannot use objects that fast.",
            ReturnValue::CanOnlyUseThisRuneOnCreatures => "You can only use it on creatures.",
            ReturnValue::PlayerIsNotReachable => "Player is not reachable.",
            ReturnValue::CreatureIsNotReachable => "Creature is not reachable.",
            ReturnValue::ActionNotPermittedInProtectionZone => {
                "This action is not permitted in a protection zone."
            }
            ReturnValue::YouMayNotAttackThisPlayer => "You may not attack this person.",
            ReturnValue::YouMayNotAttackThisCreature => "You may not attack this creature.",
            ReturnValue::YouMayNotAttackAPersonInProtectionZone => {
                "You may not attack a person in a protection zone."
            }
            ReturnValue::YouMayNotAttackAPersonWhileInProtectionZone => {
                "You may not attack a person while you are in a protection zone."
            }
            ReturnValue::YouCanOnlyUseItOnCreatures => "You can only use it on creatures.",
            ReturnValue::TurnSecureModeToAttackUnmarkedPlayers => {
                "Turn secure mode off if you really want to attack unmarked players."
            }
            ReturnValue::YouNeedPremiumAccount => "You need a premium account.",
            ReturnValue::YouNeedToLearnThisSpell => "You must learn this spell first.",
            ReturnValue::YourVocationCannotUseThisSpell => {
                "You have the wrong vocation to cast this spell."
            }
            ReturnValue::YouNeedAWeaponToUseThisSpell => {
                "You need to equip a weapon to use this spell."
            }
            ReturnValue::PlayerIsPzLockedLeavePvpZone => {
                "You can not leave a pvp zone after attacking another player."
            }
            ReturnValue::PlayerIsPzLockedEnterPvpZone => {
                "You can not enter a pvp zone after attacking another player."
            }
            ReturnValue::ActionNotPermittedInANoPvpZone => {
                "This action is not permitted in a non pvp zone."
            }
            ReturnValue::YouCannotLogoutHere => "You can not logout here.",
            ReturnValue::YouNeedAMagicItemToCastSpell => {
                "You need a magic item to cast this spell."
            }
            ReturnValue::CannotConjureItemHere => "You cannot conjure items here.",
            ReturnValue::YouNeedToSplitYourSpears => "You need to split your spears first.",
            ReturnValue::NameIsTooAmbiguous => "Player name is ambiguous.",
            ReturnValue::CanOnlyUseOneShield => "You may use only one shield.",
            ReturnValue::NoPartyMembersInRange => "No party members in range.",
            ReturnValue::YouAreNotTheOwner => "You are not the owner.",
            ReturnValue::NoSuchRaidExists => "No such raid exists.",
            ReturnValue::AnotherRaidIsAlreadyExecuting => "Another raid is already executing.",
            ReturnValue::TradePlayerFarAway => "Trade player is too far away.",
            ReturnValue::YouDontOwnThisHouse => "You don't own this house.",
            ReturnValue::TradePlayerAlreadyOwnsAHouse => "Trade player already owns a house.",
            ReturnValue::TradePlayerHighestBidder => {
                "Trade player is currently the highest bidder of an auctioned house."
            }
            ReturnValue::YouCannotTradeThisHouse => "You can not trade this house.",
            ReturnValue::YouDontHaveRequiredProfession => {
                "You don't have the required profession."
            }
            ReturnValue::CannotMoveItemIsNotStoreItem => {
                "You cannot move this item into your Store inbox as it was not bought in the Store."
            }
            ReturnValue::ItemCannotBeMovedThere => "This item cannot be moved there.",
            ReturnValue::YouCannotUseThisBed => {
                "This bed can't be used, but Premium Account players can rent houses and sleep in beds there to regain health and mana."
            }
            // `RETURNVALUE_NOTPOSSIBLE`, `RETURNVALUE_CREATUREBLOCK`, etc. — `tools.cpp` default.
            ReturnValue::NotPossible | ReturnValue::CreatureBlock => "Sorry, not possible.",
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

    /// Inventory / equip cancel text — `getReturnMessage` in `src/tools.cpp`.
    #[test]
    fn inventory_cancel_messages_match_tools_cpp() {
        assert_eq!(
            ReturnValue::CannotBeDressed.description(),
            "You cannot dress this object there."
        );
        assert_eq!(
            ReturnValue::NeedExchange.description(),
            "There is not enough room."
        );
        assert_eq!(
            ReturnValue::PutThisObjectInYourHand.description(),
            "Put this object in your hand."
        );
        assert_eq!(
            ReturnValue::DropTwoHandedItem.description(),
            "Drop the double-handed object first."
        );
        assert_eq!(
            ReturnValue::CanOnlyUseOneShield.description(),
            "You may use only one shield."
        );
        assert_eq!(
            ReturnValue::NotEnoughCapacity.description(),
            "This object is too heavy for you to carry."
        );
        assert_eq!(
            ReturnValue::CreatureBlock.description(),
            "Sorry, not possible."
        );
    }
}
