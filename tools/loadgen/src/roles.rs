//! Role mix: walk / melee / caster / rune / AoE / noise. Per-bot seeded RNG.

use std::time::{Duration, Instant};

use tfs_rust_common::Position;
use tfs_rust_common::protocol_opcodes::client;

use crate::encode::inventory_pos;
use crate::scenario::{BotRng, RoleKind, Scenario};
use crate::scheduler::{ActionKind, OpenLoop, ScheduledAction};
use crate::waypoints;

const CARDINALS: [u8; 4] = [
    client::MOVE_NORTH,
    client::MOVE_EAST,
    client::MOVE_SOUTH,
    client::MOVE_WEST,
];

pub fn bot_seed(scenario_seed: u64, bot_index: usize) -> u64 {
    scenario_seed.wrapping_add(bot_index as u64)
}

/// Pre-schedule open-loop actions for `duration_s` wall time from `start`.
/// Session records latency only after `warmup_s`; pass warmup+measure as `duration_s`.
pub fn fill_schedule(
    ol: &mut OpenLoop,
    role: RoleKind,
    scenario: &Scenario,
    start: Instant,
    rng: &mut BotRng,
    bounce_ns: bool,
    bot_index: usize,
) -> anyhow::Result<()> {
    let end = start + Duration::from_secs(scenario.duration_s.max(1));
    let walk_p = Duration::from_millis(scenario.walk_period_ms.max(1));
    let say_p = Duration::from_millis(scenario.say_period_ms.max(1));
    let n_steps = scenario
        .duration_s
        .saturating_mul(1000)
        .div_ceil(scenario.walk_period_ms.max(1))
        .max(1) as usize;
    let waypoint_ops = if bounce_ns {
        None
    } else if let Some(ref file) = scenario.waypoint_file {
        let path = waypoints::resolve_waypoint_path(file);
        let wp = waypoints::load_csv(&path)?;
        Some(waypoints::expand(&wp, n_steps, bot_index, rng))
    } else {
        None
    };
    let mut t = start;
    let mut north = true;
    let mut last_say = start;
    let mut i = 0u32;
    while t < end {
        let walk_op = if bounce_ns || role == RoleKind::Walker && scenario.name == "walk_ns" {
            if north {
                client::MOVE_NORTH
            } else {
                client::MOVE_SOUTH
            }
        } else if let Some(ref ops) = waypoint_ops {
            ops.get(i as usize)
                .copied()
                .unwrap_or(CARDINALS[rng.next_u64() as usize % CARDINALS.len()])
        } else {
            CARDINALS[rng.next_u64() as usize % CARDINALS.len()]
        };
        north = !north;
        ol.push(ScheduledAction {
            intended: t,
            kind: ActionKind::Walk(walk_op),
        });

        match role {
            RoleKind::Walker => {}
            RoleKind::Melee => {
                if i.is_multiple_of(2) {
                    ol.push(ScheduledAction {
                        intended: t,
                        kind: ActionKind::Attack(0),
                    });
                }
            }
            RoleKind::Caster => {
                if t.saturating_duration_since(last_say) >= say_p {
                    ol.push(ScheduledAction {
                        intended: t,
                        kind: ActionKind::Say(scenario.spell_words.clone()),
                    });
                    last_say = t;
                }
            }
            RoleKind::Rune | RoleKind::AoeRune => {
                if t.saturating_duration_since(last_say) >= say_p {
                    let aoe = role == RoleKind::AoeRune;
                    let slot = if aoe {
                        scenario.aoe_rune_slot
                    } else {
                        scenario.rune_slot
                    };
                    let server_id = if aoe {
                        scenario.aoe_rune_server_id
                    } else {
                        scenario.rune_server_id
                    };
                    ol.push(ScheduledAction {
                        intended: t,
                        kind: ActionKind::UseItemEx {
                            from: inventory_pos(slot),
                            from_sprite: scenario.rune_sprite_id,
                            from_server_id: server_id,
                            to_sprite: 0,
                            target_other: true,
                        },
                    });
                    last_say = t;
                }
            }
            RoleKind::Noise => {
                ol.push(ScheduledAction {
                    intended: t,
                    kind: ActionKind::LookAt(Position::new(0, 0, 0)),
                });
                if t.saturating_duration_since(last_say) >= say_p {
                    ol.push(ScheduledAction {
                        intended: t,
                        kind: ActionKind::Say("hi".into()),
                    });
                    last_say = t;
                }
            }
        }

        t += walk_p;
        i += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::RoleWeight;

    #[test]
    fn walker_schedules_walks() {
        let s = Scenario {
            name: "walk_ns".into(),
            bots: 1,
            duration_s: 1,
            warmup_s: 0,
            seed: 1,
            walk_period_ms: 200,
            say_period_ms: 2500,
            roles: vec![RoleWeight {
                kind: RoleKind::Walker,
                weight: 1.0,
            }],
            spell_words: "exori vis".into(),
            rune_sprite_id: 3155,
            rune_slot: 10,
            rune_server_id: 2268,
            aoe_rune_server_id: 2304,
            aoe_rune_slot: 6,
            waypoint_file: None,
        };
        let mut ol = OpenLoop::new();
        let mut rng = BotRng::new(1);
        fill_schedule(
            &mut ol,
            RoleKind::Walker,
            &s,
            Instant::now(),
            &mut rng,
            true,
            0,
        )
        .expect("schedule");
        assert!(!ol.is_empty());
    }
}
