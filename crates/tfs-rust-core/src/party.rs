//! Party invitations, leadership, lifecycle, shared experience split.
//!
//! Pack surface: TFS party Lua / `!share` toggle. Corpus lifecycle: `operate.cc`
//! `InviteToParty` / `RevokeInvitation` / `JoinParty` / `PassLeadership` /
//! `LeaveParty` / `DisbandParty` (`operate.hh:189-196`); marks via `GetPartyMark`
//! (`crplayer.cc:1714-1739`). Wire: TVP `SendCreatureParty` opcode `0x91`.

use tfs_rust_common::enums::ConditionType;
use tfs_rust_common::{ConnId, PlayerSex};
use tfs_rust_net::outgoing_extra::send_text_message_simple;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::login_out::creature_wire_id;
use crate::return_value::ReturnValue;

const MAX_PARTY_MEMBERS: usize = 10;
const MAX_PARTY_INVITES: usize = 10;
const MESSAGE_INFO_DESCR: u8 = 0x16;

/// 772 `PARTY_SHIELD_*` — `enums.hh`; observer-relative via `GetPartyMark`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PartyShield {
    None = 0,
    Host = 1,
    Guest = 2,
    Member = 3,
    Leader = 4,
}

impl From<PartyShield> for u8 {
    fn from(v: PartyShield) -> Self {
        v as u8
    }
}

#[derive(Debug, Clone)]
pub struct Party {
    pub id: u32,
    pub leader: CreatureId,
    pub members: Vec<CreatureId>,
    pub invited: Vec<CreatureId>,
    pub shared_experience_enabled: bool,
}

impl Party {
    pub fn new(id: u32, leader: CreatureId) -> Self {
        Self {
            id,
            leader,
            members: vec![leader],
            invited: Vec::new(),
            shared_experience_enabled: true,
        }
    }

    pub fn add_member(&mut self, c: CreatureId) {
        if !self.members.contains(&c) {
            self.members.push(c);
        }
    }

    /// Remove non-leader member. Caller must also call `Player::leave_party_marks`.
    pub fn remove_member(&mut self, c: CreatureId) -> bool {
        if c == self.leader {
            return false;
        }
        if let Some(i) = self.members.iter().position(|&x| x == c) {
            self.members.remove(i);
            return true;
        }
        false
    }

    /// Ordered removal — preserves oldest-first member order for leadership succession.
    pub fn remove_member_ordered(&mut self, c: CreatureId) -> bool {
        if let Some(i) = self.members.iter().position(|&x| x == c) {
            self.members.remove(i);
            return true;
        }
        false
    }

    /// Transfer leadership to another member (must already be in party).
    pub fn transfer_leadership(&mut self, new_leader: CreatureId) -> bool {
        if !self.members.contains(&new_leader) {
            return false;
        }
        self.leader = new_leader;
        true
    }

    pub fn is_invited(&self, guest: CreatureId) -> bool {
        self.invited.contains(&guest)
    }

    pub fn invite(&mut self, guest: CreatureId) -> bool {
        if self.invited.len() >= MAX_PARTY_INVITES || self.is_invited(guest) {
            return false;
        }
        self.invited.push(guest);
        true
    }

    pub fn revoke_invite(&mut self, guest: CreatureId) -> bool {
        if let Some(i) = self.invited.iter().position(|&x| x == guest) {
            self.invited.remove(i);
            return true;
        }
        false
    }

    pub fn accept_invite(&mut self, guest: CreatureId) -> bool {
        if let Some(i) = self.invited.iter().position(|&x| x == guest) {
            self.invited.swap_remove(i);
            self.add_member(guest);
            return true;
        }
        false
    }

    pub fn clear_invites(&mut self) -> Vec<CreatureId> {
        std::mem::take(&mut self.invited)
    }
}

/// Split `total` experience among `participants` when TFS shared XP is on.
/// 772 `DistributeExperiencePoints` has no party monster split — even divide only.
// C++ reference: `crcombat.cc:908-921` (per-attacker damage share, no party bonus).
pub fn split_shared_experience(total: u64, participants: usize) -> u64 {
    if participants == 0 {
        return 0;
    }
    total / participants as u64
}

impl GameWorld {
    /// Client `0xA3` — `InviteToParty` (`operate.cc:3919-4001`).
    pub fn player_party_invite(&mut self, conn_id: ConnId, host: CreatureId, target_wire: u32) {
        let Some(guest) = self.creature_by_wire_id(target_wire) else {
            self.send_cancel_message(conn_id, ReturnValue::PlayerWithThisNameIsNotOnline);
            return;
        };
        if host == guest {
            return;
        }
        if !matches!(self.creatures.get(guest), Some(CreatureKind::Player(_))) {
            return;
        }
        if !self.can_see_creature(host, guest) {
            return;
        }

        let host_name = self.player_name(host);
        let guest_name = self.player_name(guest);
        let host_sex = self.player_sex(host);
        let possessive = if host_sex == PlayerSex::Male {
            "his"
        } else {
            "her"
        };

        if !self.player_has_party(host, true) {
            if self.player_has_party(guest, true) {
                self.send_party_info(host, format!("{guest_name} is already member of a party."));
                return;
            }
            let party_id = self.next_party_id;
            self.next_party_id += 1;
            let mut party = Party::new(party_id, host);
            if !party.invite(guest) {
                return;
            }
            self.parties.insert(party_id, party);
            self.player_join_party(host, party_id);
            self.send_party_info(host, format!("{guest_name} has been invited."));
            self.send_party_info(
                guest,
                format!("{host_name} invites you to {possessive} party."),
            );
            self.send_party_creature_updates(host, host);
            self.send_party_creature_updates(host, guest);
            self.send_party_creature_updates(guest, host);
            return;
        }

        if !self.player_is_party_leader(host) {
            self.send_party_info(host, "You may not invite players.");
            return;
        }

        let party_id = match self.player_party_id(host) {
            Some(id) => id,
            None => return,
        };

        if self.player_has_party(guest, true) {
            let same = self.player_party_id(guest) == Some(party_id);
            let word = if same { "your" } else { "a" };
            self.send_party_info(
                host,
                format!("{guest_name} is already member of {word} party."),
            );
            return;
        }

        let invited = {
            let Some(party) = self.parties.get(&party_id) else {
                return;
            };
            if party.is_invited(guest) {
                self.send_party_info(host, format!("{guest_name} has already been invited."));
                return;
            }
            if party.members.len() >= MAX_PARTY_MEMBERS {
                return;
            }
            true
        };
        if !invited {
            return;
        }
        if let Some(party) = self.parties.get_mut(&party_id) {
            party.invite(guest);
        }
        self.send_party_info(host, format!("{guest_name} has been invited."));
        self.send_party_info(
            guest,
            format!("{host_name} invites you to {possessive} party."),
        );
        self.send_party_creature_updates(host, guest);
        self.send_party_creature_updates(guest, host);
    }

    /// Client `0xA5` — `RevokeInvitation` (`operate.cc:4003-4065`).
    pub fn player_party_revoke_invite(
        &mut self,
        _conn_id: ConnId,
        host: CreatureId,
        target_wire: u32,
    ) {
        if !self.player_is_party_leader(host) {
            self.send_party_info(host, "You may not invite players.");
            return;
        }
        let party_id = match self.player_party_id(host) {
            Some(id) => id,
            None => return,
        };
        let guest = self.creature_by_wire_id(target_wire);
        let host_name = self.player_name(host);
        let host_sex = self.player_sex(host);
        let possessive = if host_sex == PlayerSex::Male {
            "his"
        } else {
            "her"
        };

        let guest_cid = match guest {
            Some(g) => g,
            None => {
                self.send_party_info(host, "This player has not been invited.");
                return;
            }
        };
        if !self
            .parties
            .get(&party_id)
            .is_some_and(|p| p.is_invited(guest_cid))
        {
            if self.creatures.get(guest_cid).is_some() {
                let guest_name = self.player_name(guest_cid);
                self.send_party_info(host, format!("{guest_name} has not been invited."));
            } else {
                self.send_party_info(host, "This player has not been invited.");
            }
            return;
        }
        if let Some(party) = self.parties.get_mut(&party_id) {
            party.revoke_invite(guest_cid);
        }
        let guest_name = self.player_name(guest_cid);
        self.send_party_info(
            host,
            format!("Invitation for {guest_name} has been revoked."),
        );
        if self.creatures.get(guest_cid).is_some() {
            self.send_party_info(
                guest_cid,
                format!("{host_name} has revoked {possessive} invitation."),
            );
            self.send_party_creature_updates(host, guest_cid);
            self.send_party_creature_updates(guest_cid, host);
        }

        let should_disband = self
            .parties
            .get(&party_id)
            .is_some_and(|p| p.members.len() == 1 && p.invited.is_empty());
        if should_disband {
            self.disband_party(party_id);
        }
    }

    /// Client `0xA4` — `JoinParty` (`operate.cc:4067-4136`).
    pub fn player_party_join(&mut self, conn_id: ConnId, guest: CreatureId, host_wire: u32) {
        let Some(host) = self.creature_by_wire_id(host_wire) else {
            self.send_cancel_message(conn_id, ReturnValue::PlayerWithThisNameIsNotOnline);
            return;
        };
        if guest == host {
            return;
        }
        let host_name = self.player_name(host);
        let guest_name = self.player_name(guest);

        if self.player_has_party(guest, true) {
            let same = self.player_in_party_with(guest, host, true);
            let word = if same { "this" } else { "a" };
            self.send_party_info(guest, format!("You are already member of {word} party."));
            return;
        }

        let party_id = match self.find_party_id_by_leader(host) {
            Some(id) => id,
            None => {
                self.send_party_info(guest, format!("{host_name} has not invited you."));
                return;
            }
        };

        let accepted = {
            let Some(party) = self.parties.get_mut(&party_id) else {
                self.send_party_info(guest, format!("{host_name} has not invited you."));
                return;
            };
            if !party.is_invited(guest) {
                false
            } else if party.members.len() >= MAX_PARTY_MEMBERS {
                false
            } else {
                party.accept_invite(guest);
                true
            }
        };
        if !accepted {
            self.send_party_info(guest, format!("{host_name} has not invited you."));
            return;
        }

        self.player_join_party(guest, party_id);
        self.send_party_info(guest, format!("You have joined {host_name}'s party."));

        let members: Vec<CreatureId> = self
            .parties
            .get(&party_id)
            .map(|p| p.members.clone())
            .unwrap_or_default();
        for member in &members {
            self.send_party_creature_updates(guest, *member);
            if *member != guest {
                self.send_party_info(*member, format!("{guest_name} has joined the party."));
                self.send_party_creature_updates(*member, guest);
            }
        }
    }

    /// Client `0xA6` — `PassLeadership` (`operate.cc:4138-4212`).
    pub fn player_party_pass_leadership(
        &mut self,
        conn_id: ConnId,
        old_leader: CreatureId,
        new_leader_wire: u32,
    ) {
        let Some(new_leader) = self.creature_by_wire_id(new_leader_wire) else {
            self.send_cancel_message(conn_id, ReturnValue::PlayerWithThisNameIsNotOnline);
            return;
        };
        if old_leader == new_leader {
            return;
        }
        if !self.player_is_party_leader(old_leader) {
            self.send_party_info(old_leader, "You are not leader of a party.");
            return;
        }
        let party_id = match self.player_party_id(old_leader) {
            Some(id) => id,
            None => return,
        };
        let new_leader_name = self.player_name(new_leader);
        let in_party = self
            .parties
            .get(&party_id)
            .is_some_and(|p| p.members.contains(&new_leader));
        if !in_party {
            self.send_party_info(
                old_leader,
                format!("{new_leader_name} is not member of your party."),
            );
            return;
        }

        let former_invites = {
            let Some(party) = self.parties.get_mut(&party_id) else {
                return;
            };
            party.transfer_leadership(new_leader);
            party.clear_invites()
        };
        let members: Vec<CreatureId> = self
            .parties
            .get(&party_id)
            .map(|p| p.members.clone())
            .unwrap_or_default();
        for member in &members {
            if *member == new_leader {
                self.send_party_info(*member, "You are now leader of your party.");
            } else {
                self.send_party_info(
                    *member,
                    format!("{new_leader_name} is now leader of your party."),
                );
            }
            self.send_party_creature_updates(*member, old_leader);
            self.send_party_creature_updates(*member, new_leader);
        }
        for guest in former_invites {
            self.send_party_creature_updates(guest, old_leader);
            self.send_party_creature_updates(old_leader, guest);
        }
    }

    /// Client `0xA7` / forced logout — `LeaveParty` (`operate.cc:4214-4294`).
    pub fn player_party_leave(&mut self, member: CreatureId, forced: bool) {
        if !self.player_has_party(member, false) {
            return;
        }
        if !forced {
            let in_fight = self
                .creatures
                .get(member)
                .and_then(|k| match k {
                    CreatureKind::Player(p) => Some(p.earliest_logout_round > self.round_nr),
                    _ => None,
                })
                .unwrap_or(false);
            if in_fight {
                self.send_party_info(
                    member,
                    "You may not leave your party during or immediately after a fight!",
                );
                return;
            }
        }

        let party_id = match self.player_party_id(member) {
            Some(id) => id,
            None => return,
        };

        let (member_count, invite_count, is_leader) = {
            let Some(party) = self.parties.get(&party_id) else {
                return;
            };
            (
                party.members.len(),
                party.invited.len(),
                party.leader == member,
            )
        };

        if member_count == 1 || (member_count == 2 && invite_count == 0) {
            self.disband_party(party_id);
            return;
        }

        if is_leader {
            let successor = {
                let Some(party) = self.parties.get(&party_id) else {
                    return;
                };
                if party.members.first() == Some(&member) {
                    party.members.get(1).copied()
                } else {
                    party.members.first().copied()
                }
            };
            if let Some(new_leader) = successor {
                let new_wire = self.creature_wire_id_for(new_leader);
                self.player_party_pass_leadership(
                    self.conn_for_creature(member).unwrap_or(ConnId(0)),
                    member,
                    new_wire,
                );
            }
        }

        let remaining: Vec<CreatureId> = {
            let Some(party) = self.parties.get_mut(&party_id) else {
                return;
            };
            party.remove_member_ordered(member);
            party.members.clone()
        };

        let member_name = self.player_name(member);
        self.player_leave_party(member);
        if !forced {
            self.send_party_info(member, "You have left the party.");
        }
        self.send_party_creature_updates(member, member);
        for other in &remaining {
            self.send_party_creature_updates(member, *other);
            self.send_party_info(*other, format!("{member_name} has left the party."));
            self.send_party_creature_updates(*other, member);
        }
    }

    /// Client `0xA8` — TFS shared XP toggle (pack surface; not in 772 corpus).
    pub fn player_party_share_experience(&mut self, leader: CreatureId, active: bool) {
        if !self.player_is_party_leader(leader) {
            return;
        }
        let party_id = match self.player_party_id(leader) {
            Some(id) => id,
            None => return,
        };
        let in_fight = self.player_in_fight(leader);
        if let Some(party) = self.parties.get_mut(&party_id) {
            if active {
                if in_fight {
                    self.send_party_info(
                        leader,
                        "You are in fight. Experience sharing has not been enabled.",
                    );
                } else {
                    party.shared_experience_enabled = true;
                }
            } else if in_fight {
                self.send_party_info(
                    leader,
                    "You are in fight. Experience sharing has not been disabled.",
                );
            } else {
                party.shared_experience_enabled = false;
            }
        }
    }

    /// Forced leave on logout/disconnect — `crplayer.cc:311`.
    pub(crate) fn player_forced_leave_party(&mut self, cid: CreatureId) {
        self.clear_party_invites_for(cid);
        if self.player_has_party(cid, false) {
            self.player_party_leave(cid, true);
        }
    }

    fn disband_party(&mut self, party_id: u32) {
        let Some(party) = self.parties.remove(&party_id) else {
            return;
        };
        let members = party.members.clone();
        let invited = party.invited.clone();
        let leader = party.leader;

        for member in &members {
            self.player_leave_party(*member);
        }

        for member in &members {
            if self.creatures.get(*member).is_some() {
                self.send_party_info(*member, "Your party has been disbanded.");
                for other in &members {
                    self.send_party_creature_updates(*member, *other);
                }
            }
        }

        if self.creatures.get(leader).is_some() {
            for guest in &invited {
                if self.creatures.get(*guest).is_some() {
                    self.send_party_creature_updates(*guest, leader);
                    self.send_party_creature_updates(leader, *guest);
                }
            }
        }
    }

    fn clear_party_invites_for(&mut self, cid: CreatureId) {
        for party in self.parties.values_mut() {
            party.invited.retain(|&g| g != cid);
        }
    }

    pub(crate) fn find_party_id_by_leader(&self, leader: CreatureId) -> Option<u32> {
        self.parties
            .values()
            .find(|p| p.leader == leader)
            .map(|p| p.id)
    }

    pub(crate) fn player_party_id(&self, cid: CreatureId) -> Option<u32> {
        match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) if p.social.party_leaving_round == 0 => p.social.party_id,
            _ => None,
        }
    }

    pub(crate) fn player_has_party(&self, cid: CreatureId, check_former: bool) -> bool {
        match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.party_key(check_former, self.round_nr).is_some(),
            _ => false,
        }
    }

    pub(crate) fn player_is_party_leader(&self, cid: CreatureId) -> bool {
        let Some(party_id) = self.player_party_id(cid) else {
            return false;
        };
        self.parties.get(&party_id).is_some_and(|p| p.leader == cid)
    }

    pub(crate) fn player_in_party_with(
        &self,
        a: CreatureId,
        b: CreatureId,
        check_former: bool,
    ) -> bool {
        match (self.creatures.get(a), self.creatures.get(b)) {
            (Some(CreatureKind::Player(pa)), Some(CreatureKind::Player(pb))) => {
                pa.in_party_with(pb, check_former, self.round_nr)
            }
            _ => false,
        }
    }

    pub(crate) fn is_invited_to_party(&self, guest: CreatureId, host: CreatureId) -> bool {
        let Some(party_id) = self.find_party_id_by_leader(host) else {
            return false;
        };
        self.parties
            .get(&party_id)
            .is_some_and(|p| p.is_invited(guest))
    }

    pub(crate) fn player_party_leader_cid(
        &self,
        cid: CreatureId,
        check_former: bool,
    ) -> Option<CreatureId> {
        let CreatureKind::Player(p) = self.creatures.get(cid)? else {
            return None;
        };
        let party_id = p.party_key(check_former, self.round_nr)?;
        self.parties.get(&party_id).map(|party| party.leader)
    }

    fn player_in_fight(&self, cid: CreatureId) -> bool {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        if p.earliest_logout_round > self.round_nr {
            return true;
        }
        p.base
            .active_conditions
            .iter()
            .any(|c| c.ctype == ConditionType::Infight)
    }

    fn player_name(&self, cid: CreatureId) -> String {
        self.creatures
            .get(cid)
            .map(|k| k.base().name.clone())
            .unwrap_or_else(|| "Someone".to_string())
    }

    fn player_sex(&self, cid: CreatureId) -> PlayerSex {
        match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.sex,
            _ => PlayerSex::Female,
        }
    }

    fn creature_wire_id_for(&self, cid: CreatureId) -> u32 {
        match self.creatures.get(cid) {
            Some(k) => creature_wire_id(cid, k),
            None => 0,
        }
    }

    fn send_party_info(&mut self, cid: CreatureId, text: impl Into<String>) {
        let text = text.into();
        let Some(conn) = self.conn_for_creature(cid) else {
            return;
        };
        self.enqueue_outgoing(
            conn,
            send_text_message_simple(MESSAGE_INFO_DESCR, &text).into_bytes(),
        );
    }

    pub(crate) fn send_party_creature_updates(
        &mut self,
        observer: CreatureId,
        subject: CreatureId,
    ) {
        self.send_creature_shield_to_conn(subject, observer);
        self.send_creature_skull_to_conn(subject, observer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::CreatureKind;
    use crate::sim_harness::{ensure_walkable_tile, insert_player, minimal_world, test_player};
    use tfs_rust_common::{ConnId, Position};

    fn two_players() -> (GameWorld, CreatureId, CreatureId) {
        let mut world = minimal_world();
        let pos_a = Position::new(100, 100, 7);
        let pos_b = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, pos_a, 100);
        ensure_walkable_tile(&mut world.map, pos_b, 100);
        let mut pa = test_player("Alice", pos_a);
        pa.guid = 10;
        let mut pb = test_player("Bob", pos_b);
        pb.guid = 11;
        let a = insert_player(&mut world, pa);
        let b = insert_player(&mut world, pb);
        world.register_conn_mapping(ConnId(1), a);
        world.register_conn_mapping(ConnId(2), b);
        (world, a, b)
    }

    #[test]
    fn invite_and_join_adds_member() {
        let (mut world, a, b) = two_players();
        world.player_party_invite(ConnId(1), a, 11);
        assert!(world.parties.len() == 1);
        let party_id = world.player_party_id(a).expect("host in party");
        assert!(world.parties.get(&party_id).unwrap().is_invited(b));
        world.player_party_join(ConnId(2), b, 10);
        let party = world.parties.get(&party_id).unwrap();
        assert_eq!(party.members.len(), 2);
        assert!(party.members.contains(&b));
        assert_eq!(world.player_party_id(b), Some(party_id));
    }

    #[test]
    fn revoke_invite_disbands_solo_leader() {
        let (mut world, a, b) = two_players();
        world.player_party_invite(ConnId(1), a, 11);
        let party_id = world.player_party_id(a).unwrap();
        world.player_party_revoke_invite(ConnId(1), a, 11);
        assert!(!world.parties.contains_key(&party_id));
        assert!(world.player_party_id(a).is_none());
        let _ = b;
    }

    #[test]
    fn pass_leadership_clears_invites() {
        let (mut world, a, b) = two_players();
        world.player_party_invite(ConnId(1), a, 11);
        world.player_party_join(ConnId(2), b, 10);
        let mut c = test_player("Carol", Position::new(102, 100, 7));
        c.guid = 12;
        ensure_walkable_tile(&mut world.map, c.base.position, 100);
        let carol = insert_player(&mut world, c);
        world.register_conn_mapping(ConnId(3), carol);
        world.player_party_invite(ConnId(1), a, 12);
        let party_id = world.player_party_id(a).unwrap();
        world.player_party_pass_leadership(ConnId(1), a, 11);
        let party = world.parties.get(&party_id).unwrap();
        assert_eq!(party.leader, b);
        assert!(party.invited.is_empty());
    }

    #[test]
    fn leave_blocked_during_fight() {
        let (mut world, a, b) = two_players();
        world.player_party_invite(ConnId(1), a, 11);
        world.player_party_join(ConnId(2), b, 10);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(b) {
            p.earliest_logout_round = world.round_nr + 10;
        }
        world.player_party_leave(b, false);
        assert_eq!(
            world.player_party_id(b),
            Some(world.player_party_id(a).unwrap())
        );
    }

    #[test]
    fn forced_leave_on_logout() {
        let (mut world, a, b) = two_players();
        world.player_party_invite(ConnId(1), a, 11);
        world.player_party_join(ConnId(2), b, 10);
        world.player_forced_leave_party(b);
        assert!(world.player_party_id(b).is_none());
        // 772: two members + zero invites → `DisbandParty` on any leave.
        assert!(world.player_party_id(a).is_none());
        assert!(world.parties.is_empty());
    }

    #[test]
    fn split_shared_experience_even() {
        assert_eq!(split_shared_experience(100, 4), 25);
        assert_eq!(split_shared_experience(99, 4), 24);
        assert_eq!(split_shared_experience(100, 0), 0);
    }

    #[test]
    fn party_mark_leader_and_member() {
        let (mut world, a, b) = two_players();
        world.player_party_invite(ConnId(1), a, 11);
        world.player_party_join(ConnId(2), b, 10);
        assert_eq!(world.player_get_party_mark(a, b), PartyShield::Leader as u8);
        assert_eq!(world.player_get_party_mark(b, a), PartyShield::Member as u8);
    }

    /// Regression: downgraded high-level char (level 1, voc 0, ML/skills intact) killed
    /// in party must not panic when death sends the skills packet.
    ///
    /// Repro: level-100 sorcerer lowered to 1 + vocation stripped for bridge testing;
    /// ML 60 and combat skills unchanged → oversized try bars → `percent_level` overflow.
    #[tokio::test]
    async fn party_member_death_no_vocation() {
        use crate::creature::CreatureKind;
        use crate::creature::vocation::VocationProfile;

        let (mut world, killer, victim) = two_players();
        world.player_party_invite(ConnId(1), killer, 11);
        world.player_party_join(ConnId(2), victim, 10);

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(victim) {
            p.vocation_id = 0;
            p.vocation_profile = VocationProfile::none_vocation();
            p.level = 1;
            p.experience = 0;
            p.base.health = 0;
            p.base.max_health = 150;
            // Admin-downgrade left high skills on a rook shell (matches live repro).
            p.skills.maglevel = 60;
            p.skills.manaspent = 2_000_000_000_000_000_000;
            p.skills.sword = 100;
            p.base.damage_map.insert(killer, 100);
        }
        if let Some(CreatureKind::Player(k)) = world.creatures.get_mut(killer) {
            k.vocation_id = 1;
            k.level = 20;
        }

        world.apply_creature_death(victim);

        assert!(world.player_party_id(killer).is_none());
        assert!(world.parties.is_empty());
    }
}
