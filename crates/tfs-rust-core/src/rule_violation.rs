//! 772 rule-violation reports and the gamemaster request queue.
//!
//! Corpus: `CRuleViolation` / `CProcessRequest` / `CRemoveRequest` / `CCancelRequest`
//! / `CErrorFileEntry` — `receiving.cc`. Account bans stay on the existing
//! unjust-kill punishment hook; this module does not write a punishment row.

use std::collections::HashMap;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// Open help-channel request and the gamemaster currently answering it.
#[derive(Debug, Default)]
pub struct RuleRequests {
    open: HashMap<CreatureId, ()>,
    processing_gm: HashMap<CreatureId, CreatureId>,
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

    /// `CProcessRequest` (`receiving.cc:998`).
    pub(crate) fn player_process_rule_request(&mut self, cid: CreatureId, name: &str) {
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

    /// `CRemoveRequest` (`receiving.cc:1034`).
    pub(crate) fn player_remove_rule_request(&mut self, cid: CreatureId, name: &str) {
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

    /// `CCancelRequest` (`receiving.cc:1069`).
    pub(crate) fn player_cancel_rule_request(&mut self, cid: CreatureId) {
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
}
