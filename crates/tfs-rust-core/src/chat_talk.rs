//! 772 Talk ranges, RecordTalk flood, RecordMessage PM cap, trade-channel gate.
//!
//! Corpus: `Talk` — `operate.cc`; `RecordTalk` / `RecordMessage` / `CheckForMuting` —
//! `crplayer.cc`. Pack Trade channel id is **6** (`data/scripts/chatchannels/trade.lua`);
//! corpus `CHANNEL_TRADE = 5` is pack RL-Chat and must not be gated.
//!
//! Flood decision (audit Step 5): port RecordTalk for all `clientVersion` (shared
//! mechanics corpus). TFS `maxMessageBuffer` / `5n²` is not the live mute model.

use tfs_rust_common::Position;

/// Pack `Channel(6, "Trade")`. Corpus enum 5 is a different channel under TFS numbering.
pub const CHANNEL_TRADE: u16 = 6;

pub const TALK_SAY_RANGE_X: u16 = 7;
pub const TALK_SAY_RANGE_Y: u16 = 5;
pub const TALK_YELL_RANGE: u16 = 30;
pub const TALK_WHISPER_CLEAR: u16 = 1;
pub const TRADE_CHANNEL_COOLDOWN_ROUNDS: u32 = 120;
pub const RECORD_TALK_WINDOW_MS: u64 = 2500;
pub const RECORD_TALK_OVERFLOW_MS: u64 = 7500;
pub const RECORD_MESSAGE_SLOTS: usize = 20;
pub const RECORD_MESSAGE_SLOT_AGE_ROUNDS: u32 = 600;

/// 772 `TPlayer` talk-flood / PM-cap / trade-offer fields (`cr.hh`).
#[derive(Debug, Clone)]
pub struct PlayerTalkGuard {
    /// `TalkBufferFullTime` — deadline in `ServerMilliseconds`.
    pub talk_buffer_full_time: u64,
    /// `NumberOfMutings` — shared by RecordTalk and RecordMessage; never decrements.
    pub number_of_mutings: u32,
    /// `MutingEndRound` — mute until this `RoundNr` (seconds).
    pub muting_end_round: u32,
    /// `EarliestTradeChannelRound`.
    pub earliest_trade_channel_round: u32,
    /// `Addressees[20]` — recent PM targets (player guid).
    pub addressees: [u32; RECORD_MESSAGE_SLOTS],
    /// `AddresseesTimes[20]`.
    pub addressees_times: [u32; RECORD_MESSAGE_SLOTS],
}

impl Default for PlayerTalkGuard {
    fn default() -> Self {
        Self {
            talk_buffer_full_time: 0,
            number_of_mutings: 0,
            muting_end_round: 0,
            earliest_trade_channel_round: 0,
            addressees: [0; RECORD_MESSAGE_SLOTS],
            addressees_times: [0; RECORD_MESSAGE_SLOTS],
        }
    }
}

#[inline]
pub fn axis_distance(a: u16, b: u16) -> u16 {
    (a as i32 - b as i32).unsigned_abs() as u16
}

/// SAY / WHISPER hear box: `|dx|≤7 && |dy|≤5 && same Z` (`operate.cc:2372-2380`).
pub fn talk_in_say_range(speaker: Position, viewer: Position) -> bool {
    axis_distance(speaker.x, viewer.x) <= TALK_SAY_RANGE_X
        && axis_distance(speaker.y, viewer.y) <= TALK_SAY_RANGE_Y
        && speaker.z == viewer.z
}

/// YELL box: `|dx|≤30 && |dy|≤30`; multifloor only when both are on the surface
/// (`posz ≤ 7`) (`operate.cc:2385-2392`).
pub fn talk_in_yell_range(speaker: Position, viewer: Position) -> bool {
    if axis_distance(speaker.x, viewer.x) > TALK_YELL_RANGE
        || axis_distance(speaker.y, viewer.y) > TALK_YELL_RANGE
    {
        return false;
    }
    let dz = speaker.z.abs_diff(viewer.z);
    if dz > 0 && (viewer.z > 7 || speaker.z > 7) {
        return false;
    }
    true
}

/// Whisper clear-text Chebyshev ≤1 (`operate.cc:2381-2384`).
pub fn talk_whisper_clear(speaker: Position, viewer: Position) -> bool {
    axis_distance(speaker.x, viewer.x) <= TALK_WHISPER_CLEAR
        && axis_distance(speaker.y, viewer.y) <= TALK_WHISPER_CLEAR
        && speaker.z == viewer.z
}

/// `TPlayer::CheckForMuting` — remaining mute seconds (`crplayer.cc:1776-1781`).
pub fn check_for_muting(guard: &PlayerTalkGuard, round_nr: u32) -> u32 {
    guard.muting_end_round.saturating_sub(round_nr)
}

pub fn mute_duration_seconds(number_of_mutings: u32) -> u32 {
    number_of_mutings
        .saturating_mul(number_of_mutings)
        .saturating_mul(5)
}

/// `TPlayer::RecordTalk` (`crplayer.cc:1741-1755`). Returns mute seconds when this
/// talk trips mute (that talk is not broadcast); 0 otherwise.
pub fn record_talk(guard: &mut PlayerTalkGuard, server_ms: u64, round_nr: u32) -> u32 {
    if guard.talk_buffer_full_time > server_ms {
        if guard.talk_buffer_full_time > server_ms.saturating_add(RECORD_TALK_OVERFLOW_MS) {
            guard.number_of_mutings = guard.number_of_mutings.saturating_add(1);
            let muting = mute_duration_seconds(guard.number_of_mutings);
            guard.muting_end_round = round_nr.saturating_add(muting);
            muting
        } else {
            guard.talk_buffer_full_time = guard
                .talk_buffer_full_time
                .saturating_add(RECORD_TALK_WINDOW_MS);
            0
        }
    } else {
        guard.talk_buffer_full_time = server_ms.saturating_add(RECORD_TALK_WINDOW_MS);
        0
    }
}

/// `TPlayer::RecordMessage` (`crplayer.cc:1757-1781`). Returns mute seconds when
/// 20 distinct addressees are still inside the 600-round window.
pub fn record_message(guard: &mut PlayerTalkGuard, addressee_id: u32, round_nr: u32) -> u32 {
    let mut addressee_nr: Option<usize> = None;
    for i in 0..RECORD_MESSAGE_SLOTS {
        if guard.addressees[i] == addressee_id {
            addressee_nr = Some(i);
            break;
        }
        if guard.addressees[i] == 0
            || round_nr > guard.addressees_times[i].saturating_add(RECORD_MESSAGE_SLOT_AGE_ROUNDS)
        {
            addressee_nr = Some(i);
        }
    }
    match addressee_nr {
        None => {
            guard.number_of_mutings = guard.number_of_mutings.saturating_add(1);
            let muting = mute_duration_seconds(guard.number_of_mutings);
            guard.muting_end_round = round_nr.saturating_add(muting);
            muting
        }
        Some(i) => {
            guard.addressees[i] = addressee_id;
            guard.addressees_times[i] = round_nr;
            0
        }
    }
}

pub fn trade_channel_blocked(guard: &PlayerTalkGuard, round_nr: u32) -> bool {
    guard.earliest_trade_channel_round > round_nr
}

pub fn stamp_trade_channel(guard: &mut PlayerTalkGuard, round_nr: u32) {
    guard.earliest_trade_channel_round = round_nr.saturating_add(TRADE_CHANNEL_COOLDOWN_ROUNDS);
}

pub fn muted_still_text(seconds: u32) -> String {
    mute_seconds_sentence("You are still muted", seconds)
}

pub fn muted_now_text(seconds: u32) -> String {
    mute_seconds_sentence("You are muted", seconds)
}

pub fn addressed_too_many_text(seconds: u32) -> String {
    mute_seconds_sentence(
        "You have addressed too many players. You are muted",
        seconds,
    )
}

fn mute_seconds_sentence(prefix: &str, seconds: u32) -> String {
    if seconds == 1 {
        format!("{prefix} for 1 second.")
    } else {
        format!("{prefix} for {seconds} seconds.")
    }
}

/// Guild look membership clause (`operate.cc:1900-1927`). `None` when guild name empty.
pub fn guild_membership_clause(
    looking_at_self: bool,
    pronoun: &str,
    guild: &str,
    rank: &str,
    nick: &str,
) -> Option<String> {
    if guild.is_empty() {
        return None;
    }
    let lead = if looking_at_self {
        "You are ".to_string()
    } else {
        format!("{pronoun} is ")
    };
    let role = if rank.is_empty() { "a member" } else { rank };
    let mut clause = format!("{lead}{role} of the {guild}");
    if !nick.is_empty() {
        clause.push_str(" (");
        clause.push_str(nick);
        clause.push(')');
    }
    Some(clause)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: u16, y: u16, z: u8) -> Position {
        Position::new(x, y, z)
    }

    #[test]
    fn say_box_is_asymmetric_7_by_5_same_z() {
        let s = pos(100, 100, 7);
        assert!(talk_in_say_range(s, pos(107, 105, 7)));
        assert!(!talk_in_say_range(s, pos(108, 105, 7)));
        assert!(!talk_in_say_range(s, pos(107, 106, 7)));
        assert!(!talk_in_say_range(s, pos(100, 100, 6)));
    }

    #[test]
    fn yell_is_30_by_30_surface_multifloor() {
        let s = pos(100, 100, 7);
        assert!(talk_in_yell_range(s, pos(130, 130, 7)));
        assert!(!talk_in_yell_range(s, pos(131, 130, 7)));
        assert!(talk_in_yell_range(s, pos(100, 100, 5)));
        let under = pos(100, 100, 8);
        assert!(talk_in_yell_range(under, pos(100, 100, 8)));
        assert!(!talk_in_yell_range(under, pos(100, 100, 7)));
        assert!(!talk_in_yell_range(s, pos(100, 100, 8)));
    }

    #[test]
    fn whisper_clear_is_chebyshev_1() {
        let s = pos(100, 100, 7);
        assert!(talk_whisper_clear(s, pos(101, 101, 7)));
        assert!(!talk_whisper_clear(s, pos(102, 100, 7)));
    }

    #[test]
    fn record_talk_trips_on_fifth_hot_talk() {
        let mut g = PlayerTalkGuard::default();
        assert_eq!(record_talk(&mut g, 1000, 10), 0);
        assert_eq!(record_talk(&mut g, 1001, 10), 0);
        assert_eq!(record_talk(&mut g, 1002, 10), 0);
        assert_eq!(record_talk(&mut g, 1003, 10), 0);
        let mute = record_talk(&mut g, 1004, 10);
        assert_eq!(mute, 5);
        assert_eq!(g.muting_end_round, 15);
        assert_eq!(check_for_muting(&g, 10), 5);
        assert_eq!(check_for_muting(&g, 15), 0);
    }

    #[test]
    fn record_message_caps_at_20_distinct() {
        let mut g = PlayerTalkGuard::default();
        for id in 1..=20 {
            assert_eq!(record_message(&mut g, id, 50), 0);
        }
        assert_eq!(record_message(&mut g, 21, 50), 5);
        assert_eq!(
            record_message(&mut g, 1, 50),
            0,
            "same addressee reuses slot"
        );
        let mut aged = PlayerTalkGuard::default();
        for id in 1..=20 {
            assert_eq!(record_message(&mut aged, id, 50), 0);
        }
        assert_eq!(
            record_message(&mut aged, 21, 50 + RECORD_MESSAGE_SLOT_AGE_ROUNDS + 1),
            0,
            "slots older than 600 rounds are reusable"
        );
    }

    #[test]
    fn trade_stamp_is_120_rounds() {
        let mut g = PlayerTalkGuard::default();
        stamp_trade_channel(&mut g, 100);
        assert!(trade_channel_blocked(&g, 100));
        assert!(trade_channel_blocked(&g, 219));
        assert!(!trade_channel_blocked(&g, 220));
    }

    #[test]
    fn guild_clause_rank_and_nick() {
        assert_eq!(
            guild_membership_clause(true, "He", "Test Guild", "Leader", "the nick").as_deref(),
            Some("You are Leader of the Test Guild (the nick)")
        );
        assert_eq!(
            guild_membership_clause(false, "She", "Test Guild", "", "").as_deref(),
            Some("She is a member of the Test Guild")
        );
        assert!(guild_membership_clause(true, "He", "", "Leader", "").is_none());
    }

    #[test]
    fn mute_text_pluralizes() {
        assert_eq!(muted_now_text(1), "You are muted for 1 second.");
        assert_eq!(muted_now_text(5), "You are muted for 5 seconds.");
        assert_eq!(
            addressed_too_many_text(20),
            "You have addressed too many players. You are muted for 20 seconds."
        );
    }
}
