//! Parsed client game packets (after opcode byte). Mirrors `ProtocolGame` parse helpers.
// C++ reference: `src/protocolgame.cpp` `ProtocolGame::parsePacket` / `parse*` methods.

use crate::enums::Direction;
use crate::Position;

#[derive(Debug, Clone)]
pub struct SayPayload {
    pub speak_class: u8,
    pub channel_id: u16,
    pub receiver: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct UseItemPayload {
    pub pos: Position,
    pub sprite_id: u16,
    pub stack_pos: u8,
    pub index: u8,
}

#[derive(Debug, Clone)]
pub struct UseItemExPayload {
    pub from_pos: Position,
    pub from_sprite_id: u16,
    pub from_stack_pos: u8,
    pub to_pos: Position,
    pub to_sprite_id: u16,
    pub to_stack_pos: u8,
}

#[derive(Debug, Clone)]
pub struct ThrowPayload {
    pub from_pos: Position,
    pub sprite_id: u16,
    pub from_stack_pos: u8,
    pub to_pos: Position,
    pub count: u8,
}

#[derive(Debug, Clone)]
pub struct SetOutfitPayload {
    pub look_type: u16,
    pub look_head: u8,
    pub look_body: u8,
    pub look_legs: u8,
    pub look_feet: u8,
    pub look_addons: u8,
    pub look_mount: u16,
}

#[derive(Debug, Clone)]
pub struct RuleViolationPayload {
    pub report_type: u8,
    pub report_reason: u8,
    pub target_name: String,
    pub comment: String,
    pub translation: String,
    pub statement_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct BugReportPayload {
    pub category: u8,
    pub message: String,
    pub position: Option<Position>,
}

#[derive(Debug, Clone)]
pub enum GamePacket {
    /// Client finished pending → in-game (`ClientEnterGame` / `0x0F`). Server already sent map; no action.
    EnterGame,
    Logout,
    PingBack,
    Ping,
    ExtendedOpcode {
        opcode: u8,
        buffer: String,
    },
    AutoWalk {
        path: Vec<Direction>,
    },
    Move(Direction),
    StopAutoWalk,
    Turn(Direction),
    EquipObject {
        sprite_id: u16,
    },
    Throw(ThrowPayload),
    LookInShop {
        item_id: u16,
        count: u8,
    },
    PlayerPurchase {
        item_id: u16,
        count: u8,
        amount: u8,
        ignore_cap: bool,
        in_backpacks: bool,
    },
    PlayerSale {
        item_id: u16,
        count: u8,
        amount: u8,
        ignore_equipped: bool,
    },
    CloseShop,
    RequestTrade {
        pos: Position,
        sprite_id: u16,
        stack_pos: u8,
        player_id: u32,
    },
    LookInTrade {
        counter_offer: bool,
        index: u8,
    },
    AcceptTrade,
    CloseTrade,
    UseItem(UseItemPayload),
    UseItemEx(UseItemExPayload),
    UseWithCreature {
        from_pos: Position,
        sprite_id: u16,
        from_stack_pos: u8,
        creature_id: u32,
    },
    RotateItem {
        pos: Position,
        sprite_id: u16,
        stack_pos: u8,
    },
    CloseContainer {
        cid: u8,
    },
    UpArrowContainer {
        cid: u8,
    },
    UpdateContainer {
        cid: u8,
    },
    TextWindow {
        window_text_id: u32,
        new_text: String,
    },
    HouseWindow {
        door_id: u8,
        house_id: u32,
        text: String,
    },
    WrapItem {
        pos: Position,
        sprite_id: u16,
        stack_pos: u8,
    },
    LookAt {
        pos: Position,
        stack_pos: u8,
    },
    LookInBattleList {
        creature_id: u32,
    },
    JoinAggression,
    Say(SayPayload),
    RequestChannels,
    OpenChannel {
        channel_id: u16,
    },
    CloseChannel {
        channel_id: u16,
    },
    OpenPrivateChannel {
        receiver: String,
    },
    CloseNpcChannel,
    FightModes {
        raw_fight_mode: u8,
        raw_chase_mode: u8,
        raw_secure_mode: u8,
        /// OTClient v8 may send a 4th byte when `GamePVPMode` is enabled (`protocolgame.cpp` parseFightModes).
        raw_pvp_mode: u8,
    },
    Attack {
        creature_id: u32,
    },
    Follow {
        creature_id: u32,
    },
    PartyInvite {
        target_id: u32,
    },
    PartyJoin {
        target_id: u32,
    },
    PartyRevokeInvite {
        target_id: u32,
    },
    PartyPassLeadership {
        target_id: u32,
    },
    PartyLeave,
    PartyShareExperience {
        active: bool,
    },
    CreatePrivateChannel,
    ChannelInvite {
        name: String,
    },
    ChannelExclude {
        name: String,
    },
    CancelAttackAndFollow,
    UpdateTile,
    BrowseField {
        pos: Position,
    },
    SeekInContainer {
        cid: u8,
        index: u16,
    },
    RequestOutfit,
    SetOutfit(SetOutfitPayload),
    ToggleMount {
        mount: bool,
    },
    VipAdd {
        name: String,
    },
    VipRemove {
        guid: u32,
    },
    VipEdit {
        guid: u32,
        description: String,
        icon: u32,
        notify: bool,
    },
    BugReport(BugReportPayload),
    ThankYou,
    DebugAssert {
        assert_line: String,
        date: String,
        description: String,
        comment: String,
    },
    QuestLog,
    QuestLine {
        quest_id: u16,
    },
    RuleViolationReport(RuleViolationPayload),
    GetObjectInfo,
    MarketLeave,
    MarketBrowse {
        browse_id: u16,
    },
    MarketCreateOffer {
        offer_type: u8,
        sprite_id: u16,
        amount: u16,
        price: u32,
        anonymous: bool,
    },
    MarketCancelOffer {
        timestamp: u32,
        counter: u16,
    },
    MarketAcceptOffer {
        timestamp: u32,
        counter: u16,
        amount: u16,
    },
    ModalWindowAnswer {
        window_id: u32,
        button: u8,
        choice: u8,
    },
}
