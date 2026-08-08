//! 772 player-killing marks — AttackedPlayers / Aggressor / skulls / murders.
//!
//! Domain: TFS-shaped player combat marks (secure mode, skull wire).
//! Outcomes: `crplayer.cc` `IsAttackJustified` / `RecordAttack` / `RecordMurder` /
//! `CheckPlayerkilling` / `ClearPlayerkillingMarks` / `GetPlayerkillingMark`;
//! timer `crmain.cc:1102`; kill logout `crmain.cc:822`; wire `sending.cc:1045`.

use tfs_rust_common::enums::SkullType;
use tfs_rust_common::Position;
use tfs_rust_common::WorldType;
use tfs_rust_net::outgoing::send_text_message;
use tfs_rust_net::outgoing_extra::send_creature_skull;

use crate::creature::CreatureKind;
use crate::creature::Player;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::login_out::creature_wire_id;

/// 772 former-mark / former-party justification window (`crplayer.cc:1421`, `1434`, `1679`).
const FORMER_MARK_ROUNDS: u32 = 5;

/// 772 `TALK_ADMIN_MESSAGE` (`enums.hh`) — murder warning text class.
const TALK_ADMIN_MESSAGE: u8 = 18;

/// 772 `CheckPlayerkilling` result: 0 = ok, 1 = red skull, 2 = ban.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerkillingVerdict {
    None = 0,
    RedSkull = 1,
    Banishment = 2,
}

impl Player {
    /// 772 `TPlayer::IsAttacker` — `crplayer.cc:1414–1430`.
    pub(crate) fn is_attacker(&self, victim: CreatureId, check_former: bool, round_nr: u32) -> bool {
        if self.attacked_players.contains(&victim) {
            return true;
        }
        if check_former
            && self
                .former_logout_round
                .saturating_add(FORMER_MARK_ROUNDS)
                >= round_nr
            && self.former_attacked_players.contains(&victim)
        {
            return true;
        }
        false
    }

    /// 772 `TPlayer::IsAggressor` — `crplayer.cc:1432–1436`.
    pub(crate) fn is_aggressor(&self, check_former: bool, round_nr: u32) -> bool {
        self.aggressor
            || (check_former
                && self.former_aggressor
                && self
                    .former_logout_round
                    .saturating_add(FORMER_MARK_ROUNDS)
                    >= round_nr)
    }

    /// 772 `TPlayer::GetPartyLeader` key — `crplayer.cc:1678–1684` (party id stand-in).
    pub(crate) fn party_key(&self, check_former: bool, round_nr: u32) -> Option<u32> {
        if self.social.party_leaving_round == 0 {
            return self.social.party_id.filter(|&id| id != 0);
        }
        if !check_former {
            return None;
        }
        if self
            .social
            .party_leaving_round
            .saturating_add(FORMER_MARK_ROUNDS)
            >= round_nr
        {
            return self
                .social
                .former_party_id
                .or(self.social.party_id)
                .filter(|&id| id != 0);
        }
        None
    }

    /// 772 `TPlayer::InPartyWith` — `crplayer.cc:1686–1694`.
    pub(crate) fn in_party_with(
        &self,
        other: &Player,
        check_former: bool,
        round_nr: u32,
    ) -> bool {
        match (
            self.party_key(check_former, round_nr),
            other.party_key(check_former, round_nr),
        ) {
            (Some(a), Some(b)) if a == b => true,
            _ => false,
        }
    }

    /// Mark party leave for CheckFormer window — `crplayer.cc:1701–1703`.
    #[allow(dead_code)] // party system not yet wired; test-exercised
    pub(crate) fn leave_party_marks(&mut self, round_nr: u32) {
        if let Some(pid) = self.social.party_id {
            self.social.former_party_id = Some(pid);
        }
        self.social.party_leaving_round = round_nr;
        self.social.party_id = None;
    }

    /// Join / rejoin clears leave window — `crplayer.cc:1696–1699`.
    #[allow(dead_code)] // party system not yet wired; test-exercised
    pub(crate) fn join_party_marks(&mut self, party_id: u32) {
        self.social.party_id = Some(party_id);
        self.social.party_leaving_round = 0;
        self.social.former_party_id = None;
    }

    /// 772 `TPlayer::CheckPlayerkilling` — `crplayer.cc:1551–1584`.
    pub(crate) fn check_playerkilling(
        &self,
        now: i64,
        day_secs: u32,
        week_secs: u32,
        month_secs: u32,
        day_red: u32,
        week_red: u32,
        month_red: u32,
        day_ban: u32,
        week_ban: u32,
        month_ban: u32,
    ) -> PlayerkillingVerdict {
        let mut last_day = 0u32;
        let mut last_week = 0u32;
        let mut last_month = 0u32;
        for &ts in &self.murder_timestamps {
            if ts == 0 {
                continue;
            }
            let age = now.saturating_sub(ts);
            if age < i64::from(day_secs) {
                last_day += 1;
            }
            if age < i64::from(week_secs) {
                last_week += 1;
            }
            if age < i64::from(month_secs) {
                last_month += 1;
            }
        }
        if last_day >= day_ban || last_week >= week_ban || last_month >= month_ban {
            PlayerkillingVerdict::Banishment
        } else if last_day >= day_red || last_week >= week_red || last_month >= month_red {
            PlayerkillingVerdict::RedSkull
        } else {
            PlayerkillingVerdict::None
        }
    }

    /// Shift murder ring left and push `now` — `crplayer.cc:1512–1515`.
    pub(crate) fn push_murder_timestamp(&mut self, now: i64) {
        self.murder_timestamps.rotate_left(1);
        if let Some(last) = self.murder_timestamps.last_mut() {
            *last = now;
        }
    }
}

/// Encode murder timestamps for DB (≤20, drop older than 30 days).
pub fn encode_murder_timestamps(timestamps: &[i64; 20], now: i64) -> String {
    const MONTH: i64 = 30 * 24 * 60 * 60;
    timestamps
        .iter()
        .copied()
        .filter(|&ts| ts != 0 && (now - ts) < MONTH)
        .map(|ts| ts.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Decode CSV murder timestamps into a 20-slot ring (oldest → newest, right-aligned).
pub fn decode_murder_timestamps(csv: &str) -> [i64; 20] {
    let mut out = [0i64; 20];
    let parsed: Vec<i64> = csv
        .split(',')
        .filter_map(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse().ok()
            }
        })
        .collect();
    let take = parsed.len().min(20);
    let start = 20 - take;
    out[start..].copy_from_slice(&parsed[parsed.len() - take..]);
    out
}

impl GameWorld {
    /// 772 `TPlayer::IsAttackJustified` — `crplayer.cc:1438–1460`.
    pub(crate) fn player_is_attack_justified(
        &self,
        attacker: CreatureId,
        victim: CreatureId,
    ) -> bool {
        let Some(CreatureKind::Player(vic)) = self.creatures.get(victim) else {
            return true;
        };
        if self.pvp_config.world_type == WorldType::PvpEnforced || vic.playerkiller_end != 0 {
            return true;
        }
        let round_nr = self.round_nr;
        if vic.is_aggressor(true, round_nr) {
            return true;
        }
        let Some(CreatureKind::Player(atk)) = self.creatures.get(attacker) else {
            return false;
        };
        if vic.in_party_with(atk, true, round_nr) {
            return true;
        }
        vic.is_attacker(attacker, true, round_nr)
    }

    /// 772 `TPlayer::GetPlayerkillingMark` — `crplayer.cc:1650–1676`.
    pub(crate) fn player_get_killing_mark(
        &self,
        subject: CreatureId,
        observer: CreatureId,
    ) -> SkullType {
        if self.pvp_config.world_type != WorldType::Pvp {
            return SkullType::None;
        }
        let Some(CreatureKind::Player(subj)) = self.creatures.get(subject) else {
            return SkullType::None;
        };
        let Some(CreatureKind::Player(obs)) = self.creatures.get(observer) else {
            return SkullType::None;
        };
        if subj.playerkiller_end != 0 {
            return SkullType::Red;
        }
        if subj.aggressor {
            return SkullType::White;
        }
        if subj.in_party_with(obs, false, self.round_nr) {
            return SkullType::Green;
        }
        if subj.is_attacker(observer, false, self.round_nr) {
            return SkullType::Yellow;
        }
        SkullType::None
    }

    /// 772 `TPlayer::RecordAttack` — `crplayer.cc:1462–1490`.
    pub(crate) fn player_record_attack(&mut self, attacker: CreatureId, victim: CreatureId) {
        if self.pvp_config.world_type != WorldType::Pvp || attacker == victim {
            return;
        }
        if !matches!(self.creatures.get(victim), Some(CreatureKind::Player(_))) {
            return;
        }
        if !matches!(self.creatures.get(attacker), Some(CreatureKind::Player(_))) {
            return;
        }

        let round_nr = self.round_nr;
        let skip_yellow = {
            let Some(CreatureKind::Player(vic)) = self.creatures.get(victim) else {
                return;
            };
            let Some(CreatureKind::Player(atk)) = self.creatures.get(attacker) else {
                return;
            };
            vic.in_party_with(atk, true, round_nr)
                || vic.is_attacker(attacker, true, round_nr)
                || atk.is_attacker(victim, false, round_nr)
        };

        let mut send_yellow_to_victim = false;
        if !skip_yellow {
            if let Some(CreatureKind::Player(atk)) = self.creatures.get_mut(attacker) {
                if !atk.attacked_players.contains(&victim) {
                    atk.attacked_players.push(victim);
                    send_yellow_to_victim = true;
                }
            }
        }

        let unjustified = !self.player_is_attack_justified(attacker, victim);
        let mut became_aggressor = false;
        if unjustified {
            if let Some(CreatureKind::Player(atk)) = self.creatures.get_mut(attacker) {
                if !atk.aggressor {
                    atk.aggressor = true;
                    became_aggressor = true;
                }
            }
        }

        if send_yellow_to_victim {
            self.send_creature_skull_to_conn(attacker, victim);
        }
        if became_aggressor {
            self.announce_creature_skull(attacker);
        }
    }

    /// 772 `TPlayer::RecordMurder` — `crplayer.cc:1492–1535`.
    pub(crate) fn player_record_murder(&mut self, attacker: CreatureId, victim: CreatureId) -> bool {
        if self.pvp_config.world_type != WorldType::Pvp || attacker == victim {
            return false;
        }
        if self.player_is_attack_justified(attacker, victim) {
            return false;
        }
        let victim_name = match self.creatures.get(victim) {
            Some(CreatureKind::Player(p)) => p.base.name.clone(),
            _ => return false,
        };
        if !matches!(self.creatures.get(attacker), Some(CreatureKind::Player(_))) {
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let cfg = self.pvp_config;

        let (old_end, verdict) = {
            let Some(CreatureKind::Player(atk)) = self.creatures.get_mut(attacker) else {
                return false;
            };
            atk.push_murder_timestamp(now);
            let verdict = atk.check_playerkilling(
                now,
                cfg.unjust_day_secs,
                cfg.unjust_week_secs,
                cfg.unjust_month_secs,
                cfg.kills_day_red,
                cfg.kills_week_red,
                cfg.kills_month_red,
                cfg.kills_day_ban,
                cfg.kills_week_ban,
                cfg.kills_month_ban,
            );
            let old_end = atk.playerkiller_end;
            (old_end, verdict)
        };

        if let Some(conn) = self.conn_for_creature(attacker) {
            let msg = format!("Warning! The murder of {victim_name} was not justified.");
            self.enqueue_outgoing(conn, send_text_message(TALK_ADMIN_MESSAGE, &msg).into_bytes());
        }

        if verdict == PlayerkillingVerdict::None {
            return false;
        }

        if let Some(CreatureKind::Player(atk)) = self.creatures.get_mut(attacker) {
            atk.playerkiller_end = now.saturating_add(i64::from(cfg.red_skull_duration_secs));
        }
        if old_end == 0 {
            self.announce_creature_skull(attacker);
        }

        if verdict == PlayerkillingVerdict::Banishment {
            self.enqueue_unjust_kill_banishment(attacker);
            return true;
        }
        false
    }

    /// Minimal `PunishmentOrder` for excessive unjust kills — `crplayer.cc:1528–1533`.
    fn enqueue_unjust_kill_banishment(&mut self, killer: CreatureId) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(killer) else {
            return;
        };
        let account_id = p.account_id as i32;
        let guid = p.guid as i32;
        let ban_secs = i64::from(self.pvp_config.ban_days_length).saturating_mul(86_400);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let expires = now.saturating_add(ban_secs);
        let reason = "Exceeding the limit of unjustified kills by 100%.".to_string();
        let db = self.db.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(e) = tfs_rust_db::player::PlayerStore::new(&db)
                    .insert_account_ban(account_id, &reason, now, expires, guid)
                    .await
                {
                    tracing::error!(?e, account_id, "unjust-kill account ban insert failed");
                }
            });
        }
        if let Some(conn) = self.conn_for_creature(killer) {
            self.pending_idle_kick.push((conn, true));
            tracing::warn!(
                ?killer,
                account_id,
                "excessive unjust kills — account banned and kicked"
            );
        }
    }

    /// 772 `CheckAffectedPlayers` — `magic.cc:708–728`.
    pub(crate) fn check_affected_players(&self, caster: CreatureId, pos: Position) -> bool {
        if self.pvp_config.world_type != WorldType::Pvp {
            return true;
        }
        let secure = matches!(
            self.creatures.get(caster),
            Some(CreatureKind::Player(p)) if p.secure_mode
        );
        if !secure {
            return true;
        }
        let Some(tile) = self.map.get_tile(pos) else {
            return true;
        };
        for &cid in &tile.body().creatures {
            if cid == caster {
                continue;
            }
            if !matches!(self.creatures.get(cid), Some(CreatureKind::Player(_))) {
                continue;
            }
            if !self.player_is_attack_justified(caster, cid) {
                return false;
            }
        }
        true
    }

    /// RecordDeath + BlockLogout(900) arms for a player victim — `crmain.cc:822–870`.
    pub(crate) fn player_on_pvp_death_marks(&mut self, victim: CreatureId) {
        if !matches!(self.creatures.get(victim), Some(CreatureKind::Player(_))) {
            return;
        }
        let last_hit = self
            .creatures
            .get(victim)
            .and_then(|k| k.base().last_hit_by);
        let damage_map = self
            .creatures
            .get(victim)
            .map(|k| k.base().damage_map.clone())
            .unwrap_or_default();

        let attacker = last_hit;
        let responsible = attacker.and_then(|a| self.player_responsible_for_attack(a));

        if let Some(atk) = attacker {
            self.player_block_logout_white_skull(atk);
            if let Some(resp) = responsible {
                if resp != atk {
                    self.player_block_logout_white_skull(resp);
                }
            }
        }

        let murderer = responsible.filter(|&id| {
            matches!(self.creatures.get(id), Some(CreatureKind::Player(_)))
        });
        if let Some(m) = murderer {
            self.player_record_murder(m, victim);
        }

        let most_dangerous = {
            let window = self.mechanics.profile.exp_attribution_rounds;
            let round = self.round_nr;
            // 772 filters by window then picks max damage among players; ties keep lowest index.
            damage_map
                .iter_active()
                .filter(|(id, _, ts)| {
                    round.wrapping_sub(*ts) < window
                        && matches!(self.creatures.get(*id), Some(CreatureKind::Player(_)))
                })
                .fold(None, |best: Option<(CreatureId, u64)>, (id, dmg, _)| {
                    match best {
                        Some((_, best_dmg)) if dmg > best_dmg => Some((id, dmg)),
                        Some(_) => best,
                        None => Some((id, dmg)),
                    }
                })
                .map(|(id, _)| id)
        };
        if let Some(md) = most_dangerous {
            if Some(md) != murderer {
                self.player_record_murder(md, victim);
            }
        }
    }

    /// 772 `TPlayer::ClearAttacker` — `crplayer.cc:1586–1609`.
    pub(crate) fn player_clear_attacker(&mut self, subject: CreatureId, cleared: CreatureId) {
        let removed = if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(subject) {
            if let Some(idx) = p.attacked_players.iter().position(|&id| id == cleared) {
                p.attacked_players.swap_remove(idx);
                true
            } else {
                false
            }
        } else {
            false
        };
        if removed {
            self.send_creature_skull_to_conn(subject, cleared);
        }
    }

    /// 772 `TPlayer::ClearPlayerkillingMarks` — `crplayer.cc:1611–1648`.
    pub(crate) fn clear_playerkilling_marks(&mut self, cid: CreatureId) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        let was_aggressor = p.aggressor;
        let former_victims: Vec<CreatureId> = p.attacked_players.clone();

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.former_attacked_players = former_victims.clone();
            p.attacked_players.clear();
            p.former_aggressor = was_aggressor;
            p.former_logout_round = self.round_nr;
            p.aggressor = false;
        }

        if was_aggressor {
            self.announce_creature_skull(cid);
        } else {
            for victim in &former_victims {
                self.send_creature_skull_to_conn(cid, *victim);
            }
        }

        let others: Vec<CreatureId> = self
            .creatures
            .iter()
            .filter_map(|(id, k)| {
                if id != cid && matches!(k, CreatureKind::Player(_)) {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();
        for other in others {
            self.player_clear_attacker(other, cid);
        }
    }

    /// 772 `SendCreatureSkull` to one observer connection — `sending.cc:1045–1060`.
    pub(crate) fn send_creature_skull_to_conn(
        &mut self,
        subject: CreatureId,
        observer: CreatureId,
    ) {
        let Some(obs_conn) = self.creature_to_conn.get(&observer).copied() else {
            return;
        };
        let wire_id = match self.creatures.get(subject) {
            Some(k) => creature_wire_id(subject, k),
            None => return,
        };
        let known = self
            .known_creatures_by_conn
            .get(&obs_conn)
            .is_some_and(|set| set.contains(&wire_id));
        if !known {
            return;
        }
        let mark = self.player_get_killing_mark(subject, observer) as u8;
        self.enqueue_outgoing(obs_conn, send_creature_skull(wire_id, mark).into_bytes());
    }

    /// 772 `AnnounceChangedCreature(CREATURE_SKULL_CHANGED)` — per-observer mark.
    pub(crate) fn announce_creature_skull(&mut self, cid: CreatureId) {
        let Some(pos) = self.creatures.get(cid).map(|k| k.position()) else {
            return;
        };
        let wire_id = match self.creatures.get(cid) {
            Some(k) => creature_wire_id(cid, k),
            None => return,
        };
        let conns = self.spectator_conns(pos);
        let mut packets: Vec<(tfs_rust_common::ConnId, Vec<u8>)> = Vec::new();
        for conn in conns {
            let Some(observer) = self.conn_to_creature.get(&conn).copied() else {
                continue;
            };
            let known = self
                .known_creatures_by_conn
                .get(&conn)
                .is_some_and(|set| set.contains(&wire_id));
            if !known {
                continue;
            }
            let mark = self.player_get_killing_mark(cid, observer) as u8;
            packets.push((conn, send_creature_skull(wire_id, mark).into_bytes()));
        }
        for (conn, pkt) in packets {
            self.enqueue_outgoing(conn, pkt);
        }
    }

    /// Apply FormerParty window when a player leaves a party — `crplayer.cc:1701`.
    #[allow(dead_code)] // party system not yet wired; test-exercised
    pub(crate) fn player_leave_party(&mut self, cid: CreatureId) {
        let round_nr = self.round_nr;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.leave_party_marks(round_nr);
        }
    }

    /// Assign live party id and clear FormerParty window — `crplayer.cc:1696`.
    #[allow(dead_code)] // party system not yet wired; test-exercised
    pub(crate) fn player_join_party(&mut self, cid: CreatureId, party_id: u32) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.join_party_marks(party_id);
        }
    }

    /// Resolve responsible player for RecordAttack on the damage path (summon → master).
    pub(crate) fn player_responsible_for_attack(
        &self,
        attacker: CreatureId,
    ) -> Option<CreatureId> {
        match self.creatures.get(attacker) {
            Some(CreatureKind::Player(_)) => Some(attacker),
            Some(k) => k.base().master.filter(|&m| {
                matches!(self.creatures.get(m), Some(CreatureKind::Player(_)))
            }),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PvpConfig;
    use crate::creature::CreatureKind;
    use tfs_rust_common::WorldType;

    fn make_pvp_world(world_type: WorldType) -> GameWorld {
        let mut world = crate::sim_harness::minimal_world();
        world.pvp_config = PvpConfig {
            world_type,
            ..PvpConfig::defaults()
        };
        world
    }

    fn insert_player(world: &mut GameWorld, name: &str) -> CreatureId {
        let pos = tfs_rust_common::Position::new(0, 0, 7);
        let mut player = crate::sim_harness::test_player(name, pos);
        player.guid = (name.as_bytes().iter().map(|&b| b as u32).sum::<u32>())
            .wrapping_add(name.len() as u32);
        world.creatures.insert(CreatureKind::Player(player))
    }

    #[test]
    fn unjustified_attack_sets_aggressor_and_attacked_list() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        world.player_record_attack(a, b);
        let Some(CreatureKind::Player(pa)) = world.creatures.get(a) else {
            panic!("missing alice");
        };
        assert!(pa.aggressor);
        assert!(pa.attacked_players.contains(&b));
        assert_eq!(
            world.player_get_killing_mark(a, b),
            SkullType::White,
            "aggressor shows white to victim (white > yellow)"
        );
        assert_eq!(world.player_get_killing_mark(a, a), SkullType::White);
    }

    #[test]
    fn secure_mode_allows_retaliation_after_record() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        // Alice attacks Bob → Alice is aggressor / listed attacker for Bob.
        world.player_record_attack(a, b);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(b) {
            p.secure_mode = true;
        }
        // Bob may retaliate (Alice is aggressor → justified).
        assert!(!world.player_secure_mode_blocks_attack(b, a));
        // Alice attacking unmarked Charlie is blocked under secure.
        let c = insert_player(&mut world, "carol");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.secure_mode = true;
        }
        assert!(world.player_secure_mode_blocks_attack(a, c));
    }

    #[test]
    fn red_playerkiller_always_justified_and_red_mark() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(b) {
            p.playerkiller_end = i64::MAX;
        }
        assert!(world.player_is_attack_justified(a, b));
        assert_eq!(world.player_get_killing_mark(b, a), SkullType::Red);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.secure_mode = true;
        }
        assert!(!world.player_secure_mode_blocks_attack(a, b));
    }

    #[test]
    fn party_mates_justified_and_green() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.social.party_id = Some(7);
        }
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(b) {
            p.social.party_id = Some(7);
        }
        assert!(world.player_is_attack_justified(a, b));
        assert_eq!(world.player_get_killing_mark(b, a), SkullType::Green);
        // RecordAttack should not add yellow for party mates.
        world.player_record_attack(a, b);
        let Some(CreatureKind::Player(pa)) = world.creatures.get(a) else {
            panic!("missing");
        };
        assert!(!pa.aggressor);
        assert!(pa.attacked_players.is_empty());
    }

    #[test]
    fn clear_marks_copies_former_and_clears_display() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        world.player_record_attack(a, b);
        world.clear_playerkilling_marks(a);
        let Some(CreatureKind::Player(pa)) = world.creatures.get(a) else {
            panic!("missing");
        };
        assert!(!pa.aggressor);
        assert!(pa.former_aggressor);
        assert!(pa.attacked_players.is_empty());
        assert!(pa.former_attacked_players.contains(&b));
        assert_eq!(pa.former_logout_round, 100);
        // Display gone (CheckFormer=false).
        assert_eq!(world.player_get_killing_mark(a, b), SkullType::None);
        // Still justified within +5 rounds via former.
        world.round_nr = 104;
        assert!(world.player_is_attack_justified(b, a));
        world.round_nr = 106;
        assert!(!world.player_is_attack_justified(b, a));
    }

    #[test]
    fn clear_marks_removes_self_from_others_attacked_lists() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        // Bob lists Alice (Bob is aggressor vs Alice).
        world.player_record_attack(b, a);
        assert!(
            world
                .creatures
                .get(b)
                .and_then(|k| match k {
                    CreatureKind::Player(p) => Some(p.attacked_players.contains(&a)),
                    _ => None,
                })
                .unwrap_or(false)
        );
        // Clearing Alice's marks also ClearAttacker(Alice) on every player → drop from Bob.
        world.clear_playerkilling_marks(a);
        let Some(CreatureKind::Player(pb)) = world.creatures.get(b) else {
            panic!("missing");
        };
        assert!(
            !pb.attacked_players.contains(&a),
            "ClearAttacker should drop alice from bob's list"
        );
    }

    #[test]
    fn record_attack_noop_outside_open_pvp() {
        let mut world = make_pvp_world(WorldType::PvpEnforced);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        world.player_record_attack(a, b);
        let Some(CreatureKind::Player(pa)) = world.creatures.get(a) else {
            panic!("missing");
        };
        assert!(!pa.aggressor);
        assert!(pa.attacked_players.is_empty());
        assert_eq!(world.player_get_killing_mark(a, b), SkullType::None);
    }

    #[test]
    fn unjust_murder_sets_red_after_three_day_kills() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        let now = 1_700_000_000i64;
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            // Two prior murders within day window; third via RecordMurder path.
            p.murder_timestamps[18] = now - 100;
            p.murder_timestamps[19] = now - 50;
        }
        // Bypass wall-clock: call check after manual push matching RecordMurder body.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.push_murder_timestamp(now);
            let v = p.check_playerkilling(
                now,
                world.pvp_config.unjust_day_secs,
                world.pvp_config.unjust_week_secs,
                world.pvp_config.unjust_month_secs,
                world.pvp_config.kills_day_red,
                world.pvp_config.kills_week_red,
                world.pvp_config.kills_month_red,
                world.pvp_config.kills_day_ban,
                world.pvp_config.kills_week_ban,
                world.pvp_config.kills_month_ban,
            );
            assert_eq!(v, PlayerkillingVerdict::RedSkull);
            p.playerkiller_end = now + i64::from(world.pvp_config.red_skull_duration_secs);
        }
        assert_eq!(world.player_get_killing_mark(a, b), SkullType::Red);
    }

    #[test]
    fn six_day_murders_is_banishment_verdict() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let now = 1_700_000_000i64;
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            for i in 0..6 {
                p.murder_timestamps[14 + i] = now - (i as i64);
            }
            let v = p.check_playerkilling(
                now,
                world.pvp_config.unjust_day_secs,
                world.pvp_config.unjust_week_secs,
                world.pvp_config.unjust_month_secs,
                world.pvp_config.kills_day_red,
                world.pvp_config.kills_week_red,
                world.pvp_config.kills_month_red,
                world.pvp_config.kills_day_ban,
                world.pvp_config.kills_week_ban,
                world.pvp_config.kills_month_ban,
            );
            assert_eq!(v, PlayerkillingVerdict::Banishment);
        }
    }

    #[test]
    fn justified_murder_is_noop() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        world.player_record_attack(a, b); // a aggressor
        assert!(!world.player_record_murder(b, a)); // b killing a is justified
        let Some(CreatureKind::Player(pb)) = world.creatures.get(b) else {
            panic!("missing");
        };
        assert!(pb.murder_timestamps.iter().all(|&t| t == 0));
        assert_eq!(pb.playerkiller_end, 0);
    }

    #[test]
    fn former_party_justifies_within_five_rounds() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        world.player_join_party(a, 9);
        world.player_join_party(b, 9);
        world.player_leave_party(a);
        world.round_nr = 104;
        assert!(world.player_is_attack_justified(b, a));
        // Green uses CheckFormer=false → no green after leave.
        assert_eq!(world.player_get_killing_mark(a, b), SkullType::None);
        world.round_nr = 106;
        assert!(!world.player_is_attack_justified(b, a));
    }

    #[test]
    fn check_affected_players_blocks_secure_unmarked() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let pos = tfs_rust_common::Position::new(0, 0, 7);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        if let Some(k) = world.creatures.get_mut(a) {
            k.base_mut().position = pos;
        }
        if let Some(k) = world.creatures.get_mut(b) {
            k.base_mut().position = pos;
        }
        crate::sim_harness::ensure_walkable_tile(&mut world.map, pos, 100);
        if let Some(tile) = world.map.get_tile_mut(pos) {
            tile.body_mut().creatures = vec![a, b];
        }
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.secure_mode = true;
        }
        assert!(!world.check_affected_players(a, pos));
        world.player_record_attack(b, a);
        assert!(world.check_affected_players(a, pos));
    }

    #[test]
    fn encode_decode_murder_timestamps_roundtrip() {
        let now = 1_700_000_000i64;
        let mut ts = [0i64; 20];
        ts[18] = now - 10;
        ts[19] = now - 5;
        let csv = encode_murder_timestamps(&ts, now);
        assert_eq!(csv, format!("{},{}", now - 10, now - 5));
        let back = decode_murder_timestamps(&csv);
        assert_eq!(back[18], now - 10);
        assert_eq!(back[19], now - 5);
    }

    #[test]
    fn white_skull_block_logout_uses_config_rounds() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 50;
        let a = insert_player(&mut world, "alice");
        world.player_block_logout_white_skull(a);
        let Some(CreatureKind::Player(p)) = world.creatures.get(a) else {
            panic!("missing");
        };
        assert_eq!(p.earliest_logout_round, 50 + world.pvp_config.white_skull_rounds());
    }
}
