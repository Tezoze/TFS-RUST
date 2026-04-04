//! In-memory guild view (loaded with player; wars for PvP checks).
// C++ reference: `guild.cpp`, `IOGuild`.

use std::collections::{HashMap, HashSet};

use crate::ids::CreatureId;

#[derive(Debug, Clone)]
pub struct GuildRank {
    pub id: u32,
    pub name: String,
    pub level: u32,
}

#[derive(Debug, Clone)]
pub struct Guild {
    pub id: u32,
    pub name: String,
    pub motd: String,
    pub ranks: Vec<GuildRank>,
    /// Player guid -> rank id
    pub members: HashMap<u32, u32>,
}

impl Guild {
    pub fn member_rank(&self, player_guid: u32) -> Option<u32> {
        self.members.get(&player_guid).copied()
    }
}

/// Tracks mutually hostile guild pairs for `is_in_war`.
#[derive(Debug, Default)]
pub struct GuildWarTracker {
    wars: HashSet<(u32, u32)>,
}

impl GuildWarTracker {
    pub fn declare_war(&mut self, a: u32, b: u32) {
        let (x, y) = if a < b { (a, b) } else { (b, a) };
        self.wars.insert((x, y));
    }

    pub fn is_in_war(&self, guild_a: u32, guild_b: u32) -> bool {
        if guild_a == guild_b {
            return false;
        }
        let (x, y) = if guild_a < guild_b {
            (guild_a, guild_b)
        } else {
            (guild_b, guild_a)
        };
        self.wars.contains(&(x, y))
    }
}

/// Runtime guild + membership lookup by player creature.
#[derive(Debug, Default)]
pub struct GuildRegistry {
    pub guilds: HashMap<u32, Guild>,
    pub player_guild: HashMap<CreatureId, u32>,
    pub wars: GuildWarTracker,
}

impl GuildRegistry {
    pub fn register_online(&mut self, creature: CreatureId, guild_id: u32) {
        self.player_guild.insert(creature, guild_id);
    }

    pub fn unregister_online(&mut self, creature: CreatureId) {
        self.player_guild.remove(&creature);
    }

    pub fn is_in_war_players(&self, a: CreatureId, b: CreatureId) -> bool {
        let ga = self.player_guild.get(&a).copied();
        let gb = self.player_guild.get(&b).copied();
        match (ga, gb) {
            (Some(x), Some(y)) => self.wars.is_in_war(x, y),
            _ => false,
        }
    }
}
