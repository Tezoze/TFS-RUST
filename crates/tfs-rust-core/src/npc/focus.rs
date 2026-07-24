//! Talk / idle stimulus and queued-single-focus ownership.

use tfs_rust_common::enums::ConditionType;
use tfs_rust_content::npcs::{DialoguePolicy, DialogueProgram};

use super::events::{DialogueEvent, DialogueSituationKind, DialogueTrace, QueueOp};
use super::expr::{EvalContext, PlayerVocationKind};
use super::match_rule::match_dialogue_rule;
use super::react::{apply_dialogue_plan, ReactMeta};
use crate::creature::{CreatureKind, NpcActivity, QueuedNpcAddress};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::monster_ai::compute_look_toward_target;
use crate::walk::creature_turn_with_broadcast;

impl GameWorld {
    /// 772 `TNPC::TalkStimulus` — `crnonpl.cc:1682-1711`.
    pub fn npc_talk_stimulus(
        &mut self,
        npc_id: CreatureId,
        speaker: CreatureId,
        text: &str,
        trace: &mut DialogueTrace,
    ) {
        if npc_id == speaker {
            return;
        }
        let Some(CreatureKind::Npc(_)) = self.creatures.get(npc_id) else {
            return;
        };
        let Some(CreatureKind::Player(_)) = self.creatures.get(speaker) else {
            return;
        };

        let policy = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => n.runtime.policy,
            _ => return,
        };

        match policy {
            DialoguePolicy::QueuedSingleFocus => {
                self.npc_talk_queued_single_focus(npc_id, speaker, text, trace);
            }
            DialoguePolicy::PerPlayer => {
                self.npc_talk_per_player(npc_id, speaker, text, trace);
            }
        }
    }

    /// 772 `TNPC::IdleStimulus` — timeout VANISH, ADDRESSQUEUE, sleep, roam (`crnonpl.cc:1718-1808`).
    pub fn npc_idle_stimulus(&mut self, npc_id: CreatureId, trace: &mut DialogueTrace) {
        let Some(CreatureKind::Npc(npc)) = self.creatures.get(npc_id) else {
            return;
        };
        if npc.runtime.policy != DialoguePolicy::QueuedSingleFocus {
            self.npc_idle_per_player(npc_id, trace);
            return;
        }

        let round = self.round_nr;
        let tuning = self.mechanics.profile.npc;
        let timeout = tuning.conversation_timeout_rounds;
        let talking = npc.runtime.activity == NpcActivity::Talking;
        let last_talk = npc.runtime.last_talk_round;

        if talking {
            if last_talk.saturating_add(timeout) > round {
                // Still within window — keepalive Wait (`crnonpl.cc:1720-1722`).
                let _ = self.enqueue_creature_wait(npc_id, u64::from(tuning.talking_keepalive_ms));
                self.todo_start_from_action(npc_id, u64::from(tuning.talking_keepalive_ms).max(1));
                return;
            }
            // Timeout → VANISH then idle.
            let focus = npc.runtime.focus;
            if let Some(player) = focus {
                self.npc_react(
                    npc_id,
                    player,
                    "",
                    DialogueSituationKind::Vanish,
                    false,
                    trace,
                );
            }
            if let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) {
                npc.runtime.activity = NpcActivity::Idle;
                npc.runtime.focus = None;
                trace.push(DialogueEvent::State { value: "idle" });
                trace.push(DialogueEvent::Focus {
                    player: None,
                    temporary: false,
                });
            }
        }

        // Drain queue while idle.
        loop {
            let entry = {
                let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) else {
                    return;
                };
                if npc.runtime.activity != NpcActivity::Idle {
                    return;
                }
                npc.runtime.queue.pop_front()
            };
            let Some(entry) = entry else {
                break;
            };
            trace.push(DialogueEvent::Queue {
                op: QueueOp::Pop,
                player: entry.player,
                text: entry.text.clone(),
            });

            if !self.creatures.contains_key(entry.player) {
                continue;
            }
            if !self.npc_player_in_focus_range(npc_id, entry.player) {
                continue;
            }

            // C++ clears ToDo before ADDRESSQUEUE react (`crnonpl.cc:1742`).
            let _ = self.player_todo_clear(npc_id);

            if let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) {
                npc.runtime.activity = NpcActivity::Talking;
                npc.runtime.focus = Some(entry.player);
            }
            trace.push(DialogueEvent::State { value: "talking" });
            trace.push(DialogueEvent::Focus {
                player: Some(entry.player),
                temporary: false,
            });

            self.npc_react(
                npc_id,
                entry.player,
                &entry.text,
                DialogueSituationKind::AddressQueue,
                true,
                trace,
            );

            let still_talking = matches!(
                self.creatures.get(npc_id),
                Some(CreatureKind::Npc(n))
                    if matches!(n.runtime.activity, NpcActivity::Talking | NpcActivity::Leaving)
            );
            if still_talking {
                self.npc_turn_to(npc_id, entry.player, trace);
                if let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) {
                    npc.runtime.last_talk_round = self.round_nr;
                }
                return;
            }
        }

        // Sleep / roam when idle and unlocked (`crnonpl.cc:1761-1806`).
        let locked = self
            .creatures
            .get(npc_id)
            .is_some_and(|k| k.base().todo.locked);
        let idle = matches!(
            self.creatures.get(npc_id),
            Some(CreatureKind::Npc(n)) if n.runtime.activity == NpcActivity::Idle
        );
        if idle && !locked {
            self.npc_idle_roam_or_sleep(npc_id, trace);
        }
    }

    /// Sleep when no players nearby; otherwise try cardinal roam (`crnonpl.cc:1761-1806`).
    fn npc_idle_roam_or_sleep(&mut self, npc_id: CreatureId, trace: &mut DialogueTrace) {
        let tuning = self.mechanics.profile.npc;
        if !self.npc_players_in_sleep_range(npc_id) {
            if let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) {
                npc.runtime.activity = NpcActivity::Sleeping;
            }
            trace.push(DialogueEvent::State { value: "sleeping" });
            return;
        }

        let delay = u64::from(tuning.idle_roam_delay_ms);
        if self.npc_try_roam_step(npc_id) {
            let _ = self.enqueue_creature_go(npc_id);
            let _ = self.enqueue_creature_wait(npc_id, delay);
            if let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) {
                npc.runtime.next_walk = Some(self.server_ms.saturating_add(delay));
            }
            self.todo_start_from_action(npc_id, 1);
        } else {
            let _ = self.enqueue_creature_wait(npc_id, delay);
            if let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) {
                npc.runtime.next_walk = Some(self.server_ms.saturating_add(delay));
            }
            self.todo_start_from_action(npc_id, delay.max(1));
        }
    }

    /// `TFindCreatures(sleep_range, …, FIND_PLAYERS)` — any player in box (`crnonpl.cc:1762`).
    fn npc_players_in_sleep_range(&self, npc_id: CreatureId) -> bool {
        let tuning = self.mechanics.profile.npc;
        let npc_pos = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => n.base.position,
            _ => return false,
        };
        let rx = i32::from(tuning.sleep_search_range_x);
        let ry = i32::from(tuning.sleep_search_range_y);
        for (_id, k) in self.creatures.iter() {
            let CreatureKind::Player(p) = k else {
                continue;
            };
            let ppos = p.base.position;
            if ppos.z != npc_pos.z {
                continue;
            }
            if (ppos.x as i32 - npc_pos.x as i32).abs() <= rx
                && (ppos.y as i32 - npc_pos.y as i32).abs() <= ry
            {
                return true;
            }
        }
        false
    }

    /// Up to `idle_roam_attempts` random cardinal steps; queues walk on success.
    ///
    /// C++ `TNPC::IdleStimulus` roam — `crnonpl.cc:1768-1793` / `MovePossible` `:1672-1679`.
    fn npc_try_roam_step(&mut self, npc_id: CreatureId) -> bool {
        use tfs_rust_common::enums::Direction;

        let (pos, home, radius) = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => (
                n.base.position,
                n.runtime.home_position,
                n.runtime.radius,
            ),
            _ => return false,
        };
        let attempts = self.mechanics.profile.npc.idle_roam_attempts;
        const ROAM_DIRS: [Direction; 4] = [
            Direction::West,
            Direction::East,
            Direction::North,
            Direction::South,
        ];
        for _ in 0..attempts {
            let dir = ROAM_DIRS[self.parity_rand_mod(4) as usize];
            let dest = pos.offset(dir);
            if !self.npc_move_possible(npc_id, dest, home, radius) {
                continue;
            }
            if let Some(k) = self.creatures.get_mut(npc_id) {
                let base = k.base_mut();
                base.walk_queue.clear();
                base.walk_destinations.clear();
                base.walk_queue.push_back(dir);
                base.walk_destinations.push_back(dest);
                base.has_follow_path = false;
                base.force_update_follow_path = false;
            }
            return true;
        }
        false
    }

    /// 772 `TNPC::MovePossible` — `crnonpl.cc:1672-1679`.
    fn npc_move_possible(
        &self,
        npc_id: CreatureId,
        dest: tfs_rust_common::Position,
        home: tfs_rust_common::Position,
        radius: u16,
    ) -> bool {
        if dest.z != home.z {
            return false;
        }
        let r = i32::from(radius);
        if (dest.x as i32 - home.x as i32).abs() > r || (dest.y as i32 - home.y as i32).abs() > r {
            return false;
        }
        let Some(tile) = self.map.get_tile(dest) else {
            return false;
        };
        if matches!(tile, crate::tile::Tile::House(_)) {
            return false;
        }
        let chain = tile.body().map_object_chain();
        let Some(crate::tile::MapStackEntry::Ground(server_id)) = chain.first() else {
            return false;
        };
        if !self.items_db.is_terrain_bank(*server_id) || self.items_db.is_unpassable(*server_id) {
            return false;
        }
        for entry in &chain {
            match entry {
                crate::tile::MapStackEntry::Ground(sid) => {
                    if self.items_db.is_avoid_hazard(*sid) {
                        return false;
                    }
                }
                crate::tile::MapStackEntry::Item(item_id) => {
                    let Some(item) = self.items.get(*item_id) else {
                        return false;
                    };
                    let sid = item.item_type;
                    if self.items_db.is_avoid_hazard(sid) || self.items_db.is_unpassable(sid) {
                        return false;
                    }
                }
                crate::tile::MapStackEntry::Creature(cid) if *cid != npc_id => {
                    return false;
                }
                crate::tile::MapStackEntry::Creature(_) => {}
            }
        }
        true
    }

    /// Prune queue entries that are gone or out of focus range (`CreatureMoveStimulus` queue half).
    pub fn npc_prune_queue(&mut self, npc_id: CreatureId, trace: &mut DialogueTrace) {
        let tuning = self.mechanics.profile.npc;
        let npc_pos = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => n.base.position,
            _ => return,
        };
        let queue = match self.creatures.get_mut(npc_id) {
            Some(CreatureKind::Npc(npc)) => std::mem::take(&mut npc.runtime.queue),
            _ => return,
        };
        let mut kept = std::collections::VecDeque::new();
        for entry in queue {
            let keep = match self.creatures.get(entry.player) {
                Some(k) => {
                    let p = k.position();
                    p.z == npc_pos.z
                        && (p.x as i32 - npc_pos.x as i32).unsigned_abs()
                            < tuning.focus_range_x as u32
                        && (p.y as i32 - npc_pos.y as i32).unsigned_abs()
                            < tuning.focus_range_y as u32
                }
                None => false,
            };
            if keep {
                kept.push_back(entry);
            } else {
                trace.push(DialogueEvent::Queue {
                    op: QueueOp::Pop,
                    player: entry.player,
                    text: entry.text,
                });
            }
        }
        if let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) {
            npc.runtime.queue = kept;
        }
    }

    /// Movement / removal stimulus for focused interlocutor and queue (`crnonpl.cc:1811-1868`).
    pub fn npc_creature_move_stimulus(
        &mut self,
        npc_id: CreatureId,
        moved: CreatureId,
        deleted: bool,
        trace: &mut DialogueTrace,
    ) {
        self.npc_prune_queue(npc_id, trace);

        let (focus, activity) = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => (n.runtime.focus, n.runtime.activity),
            _ => return,
        };

        // Wake sleeping NPCs on nearby player move (`crnonpl.cc:1863-1867`).
        if focus.is_none() {
            if activity == NpcActivity::Sleeping && !deleted {
                if let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) {
                    npc.runtime.activity = NpcActivity::Idle;
                }
                trace.push(DialogueEvent::State { value: "idle" });
                self.creature_todo_yield(npc_id);
            }
            return;
        }
        let focus = focus.expect("checked");
        if moved != focus && moved != npc_id {
            return;
        }

        if !matches!(activity, NpcActivity::Talking | NpcActivity::Leaving) {
            return;
        }

        if !deleted && self.creatures.contains_key(focus) {
            self.npc_turn_to(npc_id, focus, trace);
        }

        if activity == NpcActivity::Talking
            && (deleted || !self.npc_player_in_focus_range(npc_id, focus))
        {
            self.npc_react(
                npc_id,
                focus,
                "",
                DialogueSituationKind::Vanish,
                false,
                trace,
            );
            if let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) {
                npc.runtime.activity = NpcActivity::Idle;
                npc.runtime.focus = None;
            }
            trace.push(DialogueEvent::State { value: "idle" });
            trace.push(DialogueEvent::Focus {
                player: None,
                temporary: false,
            });
        }
    }
}

impl GameWorld {
    fn npc_talk_queued_single_focus(
        &mut self,
        npc_id: CreatureId,
        speaker: CreatureId,
        text: &str,
        trace: &mut DialogueTrace,
    ) {
        let engaged = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => {
                n.runtime.activity == NpcActivity::Talking || !n.runtime.queue.is_empty()
            }
            _ => return,
        };

        if engaged {
            let focus = match self.creatures.get(npc_id) {
                Some(CreatureKind::Npc(n)) => n.runtime.focus,
                _ => return,
            };
            if Some(speaker) == focus {
                if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc_id) {
                    n.runtime.last_talk_round = self.round_nr;
                }
                self.npc_react(
                    npc_id,
                    speaker,
                    text,
                    DialogueSituationKind::Default,
                    false,
                    trace,
                );
            } else {
                // BUSY: temporarily swap interlocutor.
                let saved = focus;
                trace.push(DialogueEvent::Situation {
                    name: DialogueSituationKind::Busy.name(),
                });
                if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc_id) {
                    n.runtime.focus = Some(speaker);
                }
                trace.push(DialogueEvent::Focus {
                    player: Some(speaker),
                    temporary: true,
                });
                self.npc_react_inner(
                    npc_id,
                    speaker,
                    text,
                    DialogueSituationKind::Busy,
                    false,
                    false,
                    trace,
                );
                if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc_id) {
                    n.runtime.focus = saved;
                }
                trace.push(DialogueEvent::Focus {
                    player: saved,
                    temporary: false,
                });
            }
        } else {
            // ADDRESS — `ToDoClear` then Talking (`crnonpl.cc:1703-1707`).
            trace.push(DialogueEvent::Situation {
                name: DialogueSituationKind::Address.name(),
            });
            let _ = self.player_todo_clear(npc_id);
            if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc_id) {
                n.runtime.activity = NpcActivity::Talking;
                n.runtime.focus = Some(speaker);
                n.runtime.last_talk_round = self.round_nr;
            }
            trace.push(DialogueEvent::State { value: "talking" });
            trace.push(DialogueEvent::Focus {
                player: Some(speaker),
                temporary: false,
            });
            self.npc_react_inner(
                npc_id,
                speaker,
                text,
                DialogueSituationKind::Address,
                true,
                false,
                trace,
            );
            let still_talking = matches!(
                self.creatures.get(npc_id),
                Some(CreatureKind::Npc(n)) if n.runtime.activity == NpcActivity::Talking
                    || n.runtime.activity == NpcActivity::Leaving
            );
            if still_talking {
                self.npc_turn_to(npc_id, speaker, trace);
            }
        }
    }

    fn npc_talk_per_player(
        &mut self,
        npc_id: CreatureId,
        speaker: CreatureId,
        text: &str,
        trace: &mut DialogueTrace,
    ) {
        let active = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => n
                .runtime
                .player_sessions
                .get(&speaker)
                .is_some_and(|s| s.active),
            _ => return,
        };
        let situation = if active {
            DialogueSituationKind::Default
        } else {
            DialogueSituationKind::Address
        };
        if !active {
            if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc_id) {
                let session = n.runtime.player_sessions.entry(speaker).or_default();
                session.active = true;
                session.last_talk_round = self.round_nr;
            }
            trace.push(DialogueEvent::State { value: "talking" });
            trace.push(DialogueEvent::Focus {
                player: Some(speaker),
                temporary: false,
            });
        } else if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc_id) {
            if let Some(s) = n.runtime.player_sessions.get_mut(&speaker) {
                s.last_talk_round = self.round_nr;
            }
        }
        if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc_id) {
            n.runtime.focus = Some(speaker);
            n.runtime.activity = NpcActivity::Talking;
        }
        self.npc_react(npc_id, speaker, text, situation, true, trace);
        self.npc_turn_to(npc_id, speaker, trace);
    }

    fn npc_idle_per_player(&mut self, npc_id: CreatureId, trace: &mut DialogueTrace) {
        let timeout = self.mechanics.profile.npc.conversation_timeout_rounds;
        let round = self.round_nr;
        let expired: Vec<CreatureId> = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => n
                .runtime
                .player_sessions
                .iter()
                .filter(|(_, s)| s.active && s.last_talk_round.saturating_add(timeout) <= round)
                .map(|(id, _)| *id)
                .collect(),
            _ => return,
        };
        for player in expired {
            self.npc_react(
                npc_id,
                player,
                "",
                DialogueSituationKind::Vanish,
                false,
                trace,
            );
            if let Some(CreatureKind::Npc(n)) = self.creatures.get_mut(npc_id) {
                if let Some(s) = n.runtime.player_sessions.get_mut(&player) {
                    s.active = false;
                }
                if n.runtime.focus == Some(player) {
                    n.runtime.focus = None;
                }
            }
            trace.push(DialogueEvent::Focus {
                player: None,
                temporary: false,
            });
        }
    }

    fn npc_react(
        &mut self,
        npc_id: CreatureId,
        player: CreatureId,
        text: &str,
        situation: DialogueSituationKind,
        may_turn: bool,
        trace: &mut DialogueTrace,
    ) {
        self.npc_react_inner(npc_id, player, text, situation, may_turn, true, trace);
    }

    fn npc_react_inner(
        &mut self,
        npc_id: CreatureId,
        player: CreatureId,
        text: &str,
        situation: DialogueSituationKind,
        may_turn: bool,
        emit_situation: bool,
        trace: &mut DialogueTrace,
    ) {
        let _ = may_turn;
        if emit_situation {
            trace.push(DialogueEvent::Situation {
                name: situation.name(),
            });
        }

        let def_id = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => n.definition,
            _ => return,
        };
        let Some(def) = self.npcs_db.get(def_id) else {
            return;
        };
        let Some(program) = def.dialogue.as_ref() else {
            return;
        };
        let program: DialogueProgram = program.clone();
        let tuning = self.mechanics.profile.npc;
        let npc_name = def.name.clone();

        let (
            player_name,
            sex,
            level,
            hp,
            vocation,
            maglevel,
            premium,
            promoted,
            pz_block,
            poison,
            burning,
        ) = match self.creatures.get(player) {
            Some(CreatureKind::Player(p)) => {
                let poison = p
                    .base
                    .active_conditions
                    .iter()
                    .find(|c| c.ctype == ConditionType::Poison)
                    .map(|c| c.timer_rounds_left.unwrap_or(1).max(0))
                    .unwrap_or(0);
                let burning = p
                    .base
                    .active_conditions
                    .iter()
                    .find(|c| c.ctype == ConditionType::Fire)
                    .map(|c| c.timer_rounds_left.unwrap_or(1).max(0))
                    .unwrap_or(0);
                let premium = p.premium_ends_at > 0
                    && u64::from(p.premium_ends_at)
                        > std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                (
                    p.base.name.clone(),
                    match p.sex {
                        tfs_rust_common::PlayerSex::Male => 1u8,
                        tfs_rust_common::PlayerSex::Female => 2u8,
                    },
                    p.level,
                    p.base.health,
                    vocation_kind(p.vocation_id),
                    p.magic_level(),
                    premium,
                    p.vocation_id >= 5,
                    p.earliest_protection_zone_round > self.round_nr,
                    poison,
                    burning,
                )
            }
            _ => return,
        };
        let money = self.player_count_money(player).min(i32::MAX as u64) as i32;

        let (topic, price, amount, item_type, data) =
            self.npc_session_vars(npc_id, player, &program);
        let (world_pvp_enforced, world_non_pvp) = self.npc_world_pvp_flags();

        // Live inventory / quest / spell / RNG reads for this reaction.
        // Game-thread only: queries run outside overlapping `&mut` host mutation calls.
        let world_ptr = self as *const GameWorld;
        let inv = move |id: i32| -> i32 {
            if id <= 0 {
                return 0;
            }
            let Ok(item_id) = u16::try_from(id) else {
                return 0;
            };
            // SAFETY: single-threaded game loop; host mutations finish before next eval read.
            unsafe { (*world_ptr).player_get_item_type_count(player, item_id, -1) as i32 }
        };
        let quest = move |id: u32| -> i32 {
            unsafe { (*world_ptr).player_get_storage(player, id) }
        };
        let spell_k = move |id: i32| -> i32 {
            let key = id.to_string();
            unsafe {
                match (*world_ptr).creatures.get(player) {
                    Some(CreatureKind::Player(p)) => p
                        .persist
                        .as_ref()
                        .map(|b| {
                            i32::from(
                                b.spells
                                    .iter()
                                    .any(|s| s == &key || s.eq_ignore_ascii_case(&key)),
                            )
                        })
                        .unwrap_or(0),
                    _ => 0,
                }
            }
        };
        let spell_l = move |_id: i32| -> i32 { 0 };
        let mut rng = move |lo: i32, hi: i32| -> i32 {
            unsafe { (*world_ptr).parity_random(lo, hi) }
        };

        let now = chrono::Local::now();
        let (game_hour, game_minute) = {
            use chrono::Timelike;
            let h = now.hour() % 12;
            ((if h == 0 { 12 } else { h }) as u8, now.minute() as u8)
        };

        let mut ctx = EvalContext {
            topic,
            price,
            amount,
            item_type,
            data,
            captures: [-1, -1],
            player_name: player_name.as_str(),
            player_hp: hp,
            player_level: level,
            player_magic_level: maglevel,
            player_sex: sex,
            player_vocation: vocation,
            player_premium: premium,
            player_promoted: promoted,
            player_pz_block: pz_block,
            burning,
            poison,
            money,
            inventory_count: &inv,
            quest_value: &quest,
            spell_known: &spell_k,
            spell_level: &spell_l,
            rng: &mut rng,
            game_hour,
            game_minute,
            world_pvp_enforced,
            world_non_pvp,
            tuning,
        };

        let Some(matched) = match_dialogue_rule(&program, text, situation, &mut ctx) else {
            return;
        };
        trace.push(DialogueEvent::MatchRule {
            index: matched.rule_index,
        });

        let meta = ReactMeta {
            npc_id,
            npc_name: &npc_name,
        };
        let plan = apply_dialogue_plan(
            &program,
            matched,
            situation,
            player,
            text,
            &mut ctx,
            tuning,
            self,
            &meta,
            trace,
        );

        self.npc_apply_plan(npc_id, player, text, situation, &plan, &program, trace);
    }

    fn npc_session_vars(
        &self,
        npc_id: CreatureId,
        player: CreatureId,
        program: &DialogueProgram,
    ) -> (i32, i32, i32, i32, i32) {
        let Some(CreatureKind::Npc(n)) = self.creatures.get(npc_id) else {
            return (0, 0, 0, 0, 0);
        };
        if program.policy == DialoguePolicy::PerPlayer {
            if let Some(s) = n.runtime.player_sessions.get(&player) {
                return (s.topic, s.price, s.amount, s.item_type, s.data);
            }
        }
        (
            n.runtime.topic,
            n.runtime.price,
            n.runtime.amount,
            n.runtime.item_type,
            n.runtime.data,
        )
    }

    fn npc_apply_plan(
        &mut self,
        npc_id: CreatureId,
        player: CreatureId,
        text: &str,
        situation: DialogueSituationKind,
        plan: &super::react::DialoguePlan,
        program: &DialogueProgram,
        trace: &mut DialogueTrace,
    ) {
        let round = self.round_nr;

        if plan.queue_player {
            self.npc_enqueue(npc_id, player, text, trace);
        }

        if let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) {
            let per_player = program.policy == DialoguePolicy::PerPlayer;
            if per_player {
                let s = npc.runtime.player_sessions.entry(player).or_default();
                if let Some(v) = plan.topic {
                    s.topic = v;
                }
                if let Some(v) = plan.price {
                    s.price = v;
                }
                if let Some(v) = plan.amount {
                    s.amount = v;
                }
                if let Some(v) = plan.item_type {
                    s.item_type = v;
                }
                if let Some(v) = plan.data {
                    s.data = v;
                }
            } else {
                if let Some(v) = plan.topic {
                    npc.runtime.topic = v;
                }
                if let Some(v) = plan.price {
                    npc.runtime.price = v;
                }
                if let Some(v) = plan.amount {
                    npc.runtime.amount = v;
                }
                if let Some(v) = plan.item_type {
                    npc.runtime.item_type = v;
                }
                if let Some(v) = plan.data {
                    npc.runtime.data = v;
                }
            }

            if plan.go_idle {
                npc.runtime.activity = NpcActivity::Idle;
                npc.runtime.focus = None;
                if per_player {
                    if let Some(s) = npc.runtime.player_sessions.get_mut(&player) {
                        s.active = false;
                    }
                }
                trace.push(DialogueEvent::State { value: "idle" });
                trace.push(DialogueEvent::Focus {
                    player: None,
                    temporary: false,
                });
            } else if plan.deferred_idle {
                // Immediate Leaving; `ToDoChangeState(IDLE)` runs after replies (`crnonpl.cc:1219-1222`).
                npc.runtime.activity = NpcActivity::Leaving;
                trace.push(DialogueEvent::State { value: "leaving" });
            }

            if plan.start_todo && situation != DialogueSituationKind::Busy {
                let add = plan.final_talk_delay_ms / 1000;
                npc.runtime.last_talk_round = round.saturating_add(add);
            }
        }

        // Schedule Wait→Talk pairs + optional ChangeState + trailing Wait/Start (NPC-6).
        // Busy reactions do not extend LastTalk and still may speak; C++ only skips LastTalk bump.
        if plan.start_todo || !plan.replies.is_empty() || plan.deferred_idle {
            self.npc_schedule_todo_from_plan(npc_id, plan, trace);
        }
    }

    /// Enqueue `ToDoWait`/`ToDoTalk`/`ToDoChangeState`/`ToDoStart` from a dialogue plan.
    ///
    /// C++ `TBehaviourDatabase::react` — `crnonpl.cc:1087-1291`.
    fn npc_schedule_todo_from_plan(
        &mut self,
        npc_id: CreatureId,
        plan: &super::react::DialoguePlan,
        trace: &mut DialogueTrace,
    ) {
        use super::events::TodoOp;

        let first_delay = plan
            .replies
            .first()
            .map(|r| u64::from(r.delay_ms))
            .unwrap_or(0);

        for reply in &plan.replies {
            let _ = self.enqueue_creature_wait(npc_id, u64::from(reply.delay_ms));
            let _ = self.enqueue_creature_talk(npc_id, reply.text.clone());
            trace.push(DialogueEvent::Todo {
                op: TodoOp::Talk,
                delay_ms: Some(reply.delay_ms),
            });
        }

        if plan.deferred_idle {
            let _ = self.enqueue_creature_change_npc_state(npc_id, true);
        }

        if plan.start_todo {
            let final_delay = u64::from(plan.final_talk_delay_ms);
            let _ = self.enqueue_creature_wait(npc_id, final_delay);
            // Trace Wait/Start already emitted in `apply_dialogue_plan`.
            self.todo_start_from_action(npc_id, first_delay.max(1));
        } else if !plan.replies.is_empty() || plan.deferred_idle {
            self.todo_start_from_action(npc_id, first_delay.max(1));
        }
    }

    fn npc_enqueue(
        &mut self,
        npc_id: CreatureId,
        player: CreatureId,
        text: &str,
        trace: &mut DialogueTrace,
    ) {
        let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(npc_id) else {
            return;
        };
        if npc.runtime.queue.iter().any(|e| e.player == player) {
            trace.push(DialogueEvent::Queue {
                op: QueueOp::DedupeSkip,
                player,
                text: text.to_string(),
            });
            return;
        }
        npc.runtime.queue.push_back(QueuedNpcAddress {
            player,
            text: text.to_string(),
        });
        trace.push(DialogueEvent::Queue {
            op: QueueOp::Push,
            player,
            text: text.to_string(),
        });
    }

    fn npc_turn_to(&mut self, npc_id: CreatureId, target: CreatureId, trace: &mut DialogueTrace) {
        let (pos, current) = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => (n.base.position, n.base.direction),
            _ => return,
        };
        let target_pos = match self.creatures.get(target) {
            Some(k) => k.position(),
            None => return,
        };
        let new_dir = compute_look_toward_target(pos, target_pos, current);
        if new_dir != current {
            creature_turn_with_broadcast(self, npc_id, new_dir);
        }
        trace.push(DialogueEvent::TurnTo { player: target });
    }

    fn npc_player_in_focus_range(&self, npc_id: CreatureId, player: CreatureId) -> bool {
        let tuning = self.mechanics.profile.npc;
        let npc_pos = match self.creatures.get(npc_id) {
            Some(CreatureKind::Npc(n)) => n.base.position,
            _ => return false,
        };
        let p = match self.creatures.get(player) {
            Some(k) => k.position(),
            None => return false,
        };
        p.z == npc_pos.z
            && (p.x as i32 - npc_pos.x as i32).unsigned_abs() < tuning.focus_range_x as u32
            && (p.y as i32 - npc_pos.y as i32).unsigned_abs() < tuning.focus_range_y as u32
    }
}

fn vocation_kind(vocation_id: i32) -> PlayerVocationKind {
    // Best-effort mapping for property predicates; exact vocation tables are content-defined.
    match vocation_id {
        1 | 5 => PlayerVocationKind::Knight,
        2 | 6 => PlayerVocationKind::Paladin,
        3 | 7 => PlayerVocationKind::Sorcerer,
        4 | 8 => PlayerVocationKind::Druid,
        _ => PlayerVocationKind::None,
    }
}

/// Deliver normal-say talk stimuli to nearby NPCs after player SAY broadcast.
pub(crate) fn deliver_npc_say_stimuli(world: &mut GameWorld, speaker: CreatureId, text: &str) {
    let pos = match world.creatures.get(speaker) {
        Some(CreatureKind::Player(p)) => p.base.position,
        _ => return,
    };
    let candidates = super::stimulus::collect_npc_speech_candidates(world, speaker, pos);
    let mut sink = DialogueTrace::default();
    for npc_id in candidates {
        world.npc_talk_stimulus(npc_id, speaker, text, &mut sink);
    }
}

impl GameWorld {
    /// Fan out move/delete stimuli to nearby NPCs (`TNPC::CreatureMoveStimulus`).
    pub(crate) fn npc_dispatch_creature_move(
        &mut self,
        moved: CreatureId,
        old_pos: tfs_rust_common::Position,
        new_pos: tfs_rust_common::Position,
        deleted: bool,
    ) {
        let range = self.mechanics.profile.npc.focus_range_x.max(self.mechanics.profile.npc.focus_range_y)
            as u16
            + 1;
        let mut ids = Vec::new();
        for pos in [old_pos, new_pos] {
            let mut raw = Vec::new();
            self.map.grid.collect_spectators_sector_order(
                pos.x,
                pos.y,
                pos.z,
                range,
                range,
                &mut raw,
            );
            for cid in raw {
                if matches!(self.creatures.get(cid), Some(CreatureKind::Npc(_)))
                    && !ids.contains(&cid)
                {
                    ids.push(cid);
                }
            }
        }
        // Also notify the moved creature if it is an NPC (self-move).
        if matches!(self.creatures.get(moved), Some(CreatureKind::Npc(_)))
            && !ids.contains(&moved)
        {
            ids.push(moved);
        }
        let mut sink = DialogueTrace::default();
        for npc_id in ids {
            self.npc_creature_move_stimulus(npc_id, moved, deleted, &mut sink);
        }
    }

    /// Safety-net timeout tick for talking NPCs with an empty unlocked ToDo queue.
    ///
    /// Primary path is ToDo keepalive → `IdleStimulus` (`crnonpl.cc:1720-1722`). This only
    /// catches orphaned talkers (no pending Wait) so we do not double-fire with an armed batch.
    pub(crate) fn npc_tick_conversation_timeouts(&mut self) {
        let timeout = self.mechanics.profile.npc.conversation_timeout_rounds;
        let round = self.round_nr;
        let due: Vec<CreatureId> = self
            .creatures
            .iter()
            .filter_map(|(id, k)| match k {
                CreatureKind::Npc(n)
                    if n.runtime.activity == NpcActivity::Talking
                        && n.runtime.last_talk_round.saturating_add(timeout) <= round
                        && !n.base.todo.locked
                        && n.base.todo.is_empty() =>
                {
                    Some(id)
                }
                _ => None,
            })
            .collect();
        let mut sink = DialogueTrace::default();
        for npc_id in due {
            self.npc_idle_stimulus(npc_id, &mut sink);
        }
    }
}
