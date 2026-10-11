//! 772 rule-violation reports and the gamemaster request queue.
//!
//! Corpus: `CRuleViolation` / `CProcessRequest` / `CRemoveRequest` / `CCancelRequest`
//! / `CErrorFileEntry` — `receiving.cc`. Account bans stay on the existing
//! unjust-kill punishment hook; this module does not write a punishment row.

use std::collections::HashMap;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// One 8.0 rule-channel report (`800src/game.cpp` `RuleViolation`).
#[derive(Debug, Clone)]
struct ChannelReport {
    text: String,
    pending: bool,
    gamemaster: Option<CreatureId>,
}

/// Open help-channel request and the gamemaster currently answering it.
/// `channel` is the 8.0 `CHANNEL_RULE_REP` queue; the maps above stay the 772 path.
#[derive(Debug, Default)]
pub struct RuleRequests {
    open: HashMap<CreatureId, ()>,
    processing_gm: HashMap<CreatureId, CreatureId>,
    channel: HashMap<CreatureId, ChannelReport>,
}

impl RuleRequests {
    fn is_open(&self, cid: CreatureId) -> bool {
        self.open.contains_key(&cid)
    }

    fn processing(&self, cid: CreatureId) -> Option<CreatureId> {
        self.processing_gm.get(&cid).copied()
    }

    fn open_request(&mut self, cid: CreatureId) {
        self.open.insert(cid, ());
        self.processing_gm.remove(&cid);
    }

    fn set_processing(&mut self, target: CreatureId, gm: CreatureId) {
        self.processing_gm.insert(target, gm);
    }

    fn clear(&mut self, cid: CreatureId) {
        self.open.remove(&cid);
        self.processing_gm.remove(&cid);
    }
}

impl GameWorld {
    /// `TALK_GAMEMASTER_REQUEST` (`receiving.cc:888`).
    pub(crate) fn open_gamemaster_request(&mut self, cid: CreatureId, _text: &str) {
        if self.rule_requests.is_open(cid) {
            self.rule_fail(
                cid,
                "You have already submitted a request. Please wait until it is answered.",
            );
            return;
        }
        self.rule_requests.open_request(cid);
    }

    /// `CRuleViolation` (`receiving.cc:1532`). Logs the report; does not insert a ban.
    pub(crate) fn player_classic_rule_violation(
        &mut self,
        cid: CreatureId,
        name: &str,
        reason: u8,
        action: u8,
        comment: &str,
        statement_id: u32,
        ip_banishment: bool,
    ) {
        // `CheckBanishmentRight` and `IP_BANISHMENT` both map to pack group `access`.
        if !self.reporter_may_banish(cid) {
            self.rule_fail(cid, "You have no authorization for this action.");
            return;
        }
        if comment.is_empty() {
            self.rule_fail(cid, "Please provide a comment for this report.");
            return;
        }
        if action == 6 && statement_id == 0 {
            self.rule_fail(cid, "Please provide a statement for this rule violation.");
            return;
        }
        let Some(target) = self.find_player_by_name(name) else {
            self.rule_fail(cid, "A player with this name does not exist.");
            return;
        };
        let target_name = self
            .creatures
            .get(target)
            .map(|k| k.base().name.clone())
            .unwrap_or_else(|| name.to_string());
        tracing::info!(
            ?cid,
            target = %target_name,
            reason,
            action,
            statement_id,
            ip_banishment,
            "rule violation report"
        );
    }

    /// `CProcessRequest` (`receiving.cc:998`). 8.0 uses the rule channel instead.
    pub(crate) fn player_process_rule_request(&mut self, cid: CreatureId, name: &str) {
        if self.rule_channel_era() {
            self.process_rule_channel(cid, name);
            return;
        }
        if !self.reporter_may_banish(cid) || name.is_empty() {
            return;
        }
        let Some(target) = self.find_player_by_name(name) else {
            self.rule_fail(cid, "Player is not online any more.");
            return;
        };
        if !self.rule_requests.is_open(target) || self.rule_requests.processing(target).is_some() {
            self.rule_fail(cid, "Request has already been processed.");
            return;
        }
        self.rule_requests.set_processing(target, cid);
    }

    /// `CRemoveRequest` (`receiving.cc:1034`). 8.0 closes the named reporter's channel report.
    pub(crate) fn player_remove_rule_request(&mut self, cid: CreatureId, name: &str) {
        if self.rule_channel_era() {
            if let Some(target) = self.find_player_by_name(name) {
                self.close_rule_channel(target);
            }
            let _ = cid;
            return;
        }
        if !self.reporter_may_banish(cid) || name.is_empty() {
            return;
        }
        let Some(target) = self.find_player_by_name(name) else {
            self.rule_fail(cid, "Player is not online any more.");
            return;
        };
        if !self.rule_requests.is_open(target) {
            return;
        }
        let processing = self.rule_requests.processing(target);
        if processing.is_none() || processing == Some(cid) {
            self.rule_requests.clear(target);
        }
    }

    /// `CCancelRequest` (`receiving.cc:1069`). 8.0 is the reporter cancelling their own report.
    pub(crate) fn player_cancel_rule_request(&mut self, cid: CreatureId) {
        if self.rule_channel_era() {
            self.cancel_rule_channel(cid);
            return;
        }
        if !self.reporter_may_banish(cid) {
            return;
        }
        if self.rule_requests.is_open(cid) {
            self.rule_requests.clear(cid);
        }
    }

    /// `CErrorFileEntry` (`receiving.cc:1627`) — log only.
    pub(crate) fn log_client_error_file(
        &self,
        cid: CreatureId,
        title: &str,
        date: &str,
        stack: &str,
        comment: &str,
    ) {
        if stack.is_empty() && comment.is_empty() {
            return;
        }
        let name = self
            .creatures
            .get(cid)
            .map(|k| k.base().name.clone())
            .unwrap_or_default();
        let pos = self.creatures.get(cid).map(|k| k.position());
        tracing::info!(
            player = %name,
            ?pos,
            title,
            date,
            stack,
            comment,
            "client-error"
        );
    }

    fn reporter_may_banish(&self, cid: CreatureId) -> bool {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        self.groups
            .groups
            .get(&p.group_id)
            .is_some_and(|g| g.access)
    }

    fn find_player_by_name(&self, name: &str) -> Option<CreatureId> {
        self.player_by_name
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, id)| *id)
    }

    pub(crate) fn rule_channel_era(&self) -> bool {
        (800..1098).contains(&self.codec.version().raw())
    }

    /// `Game::playerReportRuleViolationReport` — `800src/game.cpp`.
    pub(crate) fn player_report_rule_channel(&mut self, cid: CreatureId, text: &str) {
        if self.rule_requests.channel.contains_key(&cid) {
            self.rule_fail(
                cid,
                "You already have a pending rule violation report. Close it before starting a new one.",
            );
            return;
        }
        self.rule_requests.channel.insert(
            cid,
            ChannelReport {
                text: text.to_string(),
                pending: true,
                gamemaster: None,
            },
        );
        self.fanout_rule_channel_speech(cid, text);
    }

    /// `Game::playerContinueRuleViolationReport`.
    pub(crate) fn player_continue_rule_channel(&mut self, cid: CreatureId, text: &str) {
        let gm = self
            .rule_requests
            .channel
            .get(&cid)
            .and_then(|r| r.gamemaster);
        let Some(gm) = gm else {
            return;
        };
        let (name, level, pos) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => {
                (p.base.name.clone(), p.level.max(0) as u16, p.base.position)
            }
            _ => return,
        };
        let Some(conn) = self.conn_for_creature(gm) else {
            return;
        };
        let wire = tfs_rust_net::codec::wire::CreatureSayWire {
            speaker_name: name,
            level,
            speak_type: 8, // TALKTYPE_RVR_CONTINUE
            pos,
            text: text.to_string(),
        };
        let msg = self.codec.encode_creature_say(0, &wire);
        self.enqueue_outgoing(conn, msg.into_bytes());
        self.rule_status(cid, "Message sent to Counsellor.");
    }

    /// `Game::playerProcessRuleViolationReport`.
    fn process_rule_channel(&mut self, cid: CreatureId, name: &str) {
        if !self.account_is_gamemaster(cid) {
            return;
        }
        let Some(target) = self.find_player_by_name(name) else {
            return;
        };
        let Some(report) = self.rule_requests.channel.get_mut(&target) else {
            return;
        };
        if !report.pending {
            return;
        }
        report.gamemaster = Some(cid);
        report.pending = false;
        let reporter_name = name.to_string();
        self.fanout_rule_channel_packet(encode_remove_rule_report(&reporter_name));
    }

    /// `Game::closeRuleViolationReport`.
    fn close_rule_channel(&mut self, reporter: CreatureId) {
        if self.rule_requests.channel.remove(&reporter).is_none() {
            return;
        }
        let name = self
            .creatures
            .get(reporter)
            .map(|k| k.base().name.clone())
            .unwrap_or_default();
        if let Some(conn) = self.conn_for_creature(reporter) {
            self.enqueue_outgoing(conn, encode_lock_rule_report());
        }
        self.fanout_rule_channel_packet(encode_remove_rule_report(&name));
    }

    /// `Game::cancelRuleViolationReport`.
    fn cancel_rule_channel(&mut self, reporter: CreatureId) {
        let Some(report) = self.rule_requests.channel.remove(&reporter) else {
            return;
        };
        let name = self
            .creatures
            .get(reporter)
            .map(|k| k.base().name.clone())
            .unwrap_or_default();
        if !report.pending
            && let Some(gm) = report.gamemaster
            && let Some(conn) = self.conn_for_creature(gm)
        {
            self.enqueue_outgoing(conn, encode_cancel_rule_report(&name));
        }
        self.fanout_rule_channel_packet(encode_remove_rule_report(&name));
    }

    /// `ProtocolGame::sendRuleViolationsChannel` (`0xAE`).
    pub(crate) fn encode_open_rule_channel(&self, channel_id: u16) -> Vec<u8> {
        let mut pending = Vec::new();
        for (cid, report) in &self.rule_requests.channel {
            if !report.pending {
                continue;
            }
            let Some(CreatureKind::Player(p)) = self.creatures.get(*cid) else {
                continue;
            };
            pending.push((p.base.name.clone(), report.text.clone()));
        }
        encode_rule_violations_channel(channel_id, &pending)
    }

    fn fanout_rule_channel_speech(&mut self, cid: CreatureId, text: &str) {
        let (name, level) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => (p.base.name.clone(), p.level.max(0) as u16),
            _ => return,
        };
        let users: Vec<CreatureId> = self
            .chat
            .get_channel(crate::chat::CHANNEL_RULE_REP)
            .map(|ch| ch.users.iter().copied().collect())
            .unwrap_or_default();
        let wire = tfs_rust_net::ToChannelWire {
            speaker_name: Some(name),
            level,
            speak_type: 6, // TALKTYPE_RVR_CHANNEL
            channel_id: crate::chat::CHANNEL_RULE_REP,
            text: text.to_string(),
        };
        let msg = self.codec.encode_to_channel(0, &wire).into_bytes();
        for member in users {
            if let Some(conn) = self.conn_for_creature(member) {
                self.enqueue_outgoing(conn, msg.clone());
            }
        }
    }

    fn fanout_rule_channel_packet(&mut self, packet: Vec<u8>) {
        let users: Vec<CreatureId> = self
            .chat
            .get_channel(crate::chat::CHANNEL_RULE_REP)
            .map(|ch| ch.users.iter().copied().collect())
            .unwrap_or_default();
        for member in users {
            if let Some(conn) = self.conn_for_creature(member) {
                self.enqueue_outgoing(conn, packet.clone());
            }
        }
    }

    fn account_is_gamemaster(&self, cid: CreatureId) -> bool {
        matches!(
            self.creatures.get(cid),
            Some(CreatureKind::Player(p)) if p.account_type >= 4
        )
    }

    fn rule_status(&mut self, cid: CreatureId, text: &str) {
        let Some(conn) = self.conn_for_creature(cid) else {
            return;
        };
        const MESSAGE_STATUS_SMALL: u8 = 0x17;
        let msg =
            tfs_rust_net::outgoing_extra::send_text_message_simple(MESSAGE_STATUS_SMALL, text);
        self.enqueue_outgoing(conn, msg.into_bytes());
    }

    fn rule_fail(&mut self, cid: CreatureId, text: &str) {
        let Some(conn) = self.conn_for_creature(cid) else {
            return;
        };
        let msg = tfs_rust_net::outgoing_extra::send_text_message_simple(
            self.codec.failure_message_type(),
            text,
        );
        self.enqueue_outgoing(conn, msg.into_bytes());
    }
}

/// `ProtocolGame::sendRuleViolationsChannel` — `800src/protocolgame.cpp` ~1225.
/// Each pending report is an embedded `0xAA` with no level field.
pub(crate) fn encode_rule_violations_channel(
    channel_id: u16,
    pending: &[(String, String)],
) -> Vec<u8> {
    let mut msg = tfs_rust_net::NetworkMessage::new();
    msg.write_u8(0xAE);
    msg.write_u16(channel_id);
    for (name, text) in pending {
        msg.write_u8(0xAA);
        msg.write_u32(0);
        msg.write_string(name);
        msg.write_u8(6);
        msg.write_u32(0);
        msg.write_string(text);
    }
    msg.into_bytes()
}

fn encode_remove_rule_report(name: &str) -> Vec<u8> {
    let mut msg = tfs_rust_net::NetworkMessage::new();
    msg.write_u8(0xAF);
    msg.write_string(name);
    msg.into_bytes()
}

fn encode_cancel_rule_report(name: &str) -> Vec<u8> {
    let mut msg = tfs_rust_net::NetworkMessage::new();
    msg.write_u8(0xB0);
    msg.write_string(name);
    msg.into_bytes()
}

fn encode_lock_rule_report() -> Vec<u8> {
    let mut msg = tfs_rust_net::NetworkMessage::new();
    msg.write_u8(0xB1);
    msg.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{beat_driven_test_world, insert_spectator_player, test_player};
    use tfs_rust_common::{ConnId, Position};
    use tfs_rust_content::groups::{Group, GroupDatabase};

    fn gm_world() -> (GameWorld, CreatureId) {
        let mut world = beat_driven_test_world();
        let mut groups = GroupDatabase::default();
        groups.groups.insert(
            1,
            Group {
                id: 1,
                name: "god".into(),
                access: true,
                max_depot_items: 0,
                max_vip_entries: 0,
                flags: Default::default(),
            },
        );
        world.groups = std::sync::Arc::new(groups);
        let pos = Position::new(100, 100, 7);
        let cid = insert_spectator_player(&mut world, ConnId(1), test_player("God", pos));
        world.player_by_name.insert("God".into(), cid);
        (world, cid)
    }

    #[test]
    fn empty_comment_returns_corpus_text() {
        let (mut world, cid) = gm_world();
        world.player_classic_rule_violation(cid, "God", 0, 0, "", 0, false);
        let pkts = world.pending_outgoing.get(&ConnId(1)).expect("packet");
        let needle = b"Please provide a comment for this report.";
        assert!(
            pkts.iter()
                .any(|p| p.windows(needle.len()).any(|w| w == needle)),
            "empty comment must use the corpus failure text"
        );
    }

    #[test]
    fn rule_channel_open_lists_pending_without_a_level() {
        let bytes = encode_rule_violations_channel(2, &[("Ann".into(), "help".into())]);
        assert_eq!(
            bytes,
            vec![
                0xAE, 2, 0, // channel
                0xAA, 0, 0, 0, 0, // statement
                3, 0, b'A', b'n', b'n', 6, 0, 0, 0, 0, 4, 0, b'h', b'e', b'l', b'p',
            ]
        );
        let removed = encode_remove_rule_report("Ann");
        assert_eq!(removed, vec![0xAF, 3, 0, b'A', b'n', b'n']);
    }
}
