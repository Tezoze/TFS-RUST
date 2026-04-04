//! Party invitations, leadership, shared experience split.
// C++ reference: `party.cpp` — Australis may customize XP; formula isolated below.

use std::collections::HashMap;

use crate::ids::CreatureId;

#[derive(Debug, Clone)]
pub struct Party {
    pub id: u32,
    pub leader: CreatureId,
    pub members: Vec<CreatureId>,
    pub shared_experience_enabled: bool,
}

impl Party {
    pub fn new(id: u32, leader: CreatureId) -> Self {
        Self {
            id,
            leader,
            members: vec![leader],
            shared_experience_enabled: true,
        }
    }

    pub fn add_member(&mut self, c: CreatureId) {
        if !self.members.contains(&c) {
            self.members.push(c);
        }
    }

    pub fn remove_member(&mut self, c: CreatureId) -> bool {
        if c == self.leader {
            return false;
        }
        if let Some(i) = self.members.iter().position(|&x| x == c) {
            self.members.swap_remove(i);
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
}

#[derive(Debug, Default)]
pub struct PartyInviteState {
    /// invitee -> inviter
    pub pending: HashMap<CreatureId, CreatureId>,
}

/// Split `total` experience among `participants` when shared XP is on.
/// Australis custom curve: bonus for larger parties — stub uses even split + small bonus.
// C++ reference: `Party::shareExperience` — replace with exact C++ when porting.
pub fn split_shared_experience(total: u64, participants: usize) -> u64 {
    if participants == 0 {
        return 0;
    }
    let n = participants as u64;
    let bonus = (total / 20).min(50 * n);
    (total + bonus) / n
}
