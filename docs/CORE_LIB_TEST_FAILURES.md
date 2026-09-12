# `tfs-rust-core` lib test failures (snapshot)

**Captured:** 2026-09-12 during Sim harness Phase 3 verification  
**Command:** `/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core`  
**Result:** 1421 passed, **29 failed**, 2 ignored  
**Not caused by** the `sim_harness` → `test_support` / `sim_scenario` split (those module tests passed). Treat as a later fix list.

Re-run one cluster:

```
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core --lib conjure::
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core --lib npc::tests::
```

---

## Inventory / item create (19)

Shared shape: conjure/shop/NPC `Create` and backpack hydrate see **empty slots or count 0**. Likely one fixture/item-db/container-registry gap, not 19 independent bugs.

| # | Test | Panic |
|---|------|--------|
| 1 | `conjure::tests::charge_fallback_when_count_omitted` | `conjure.rs:330` `.expect("left")` — left hand empty |
| 2 | `conjure::tests::dual_hand_skips_second_when_mana_short` | `conjure.rs:379` `.expect("left")` |
| 3 | `conjure::tests::left_hand_reagent_succeeds` | `conjure.rs:359` `.expect("left")` |
| 4 | `conjure::tests::reagent_zero_adds_to_inventory` | `conjure.rs:408` `.expect("arrows")` |
| 5 | `game_world_inventory::look_tests::add_item_full_backpack_hydrates_container_registry` | `game_world_inventory.rs:2248` `.expect("backpack id")` |
| 6 | `game_world_lua_tools::tests::lua_script_game_create_item_hydrates_container_for_add_item` | `game_world_lua_tools.rs:464` — CreateItem did not hydrate Container |
| 7 | `game_world_lua_tools::tests::r3_add_item_ex_into_detached_container` | `game_world_lua_tools.rs:558` left `1` right `0` |
| 8 | `npc::tests::bank_change_delete_then_create_ordering` | `npc/tests.rs:684` `saw_delete && saw_create` |
| 9 | `npc::tests::create_cumulative_chunks_by_100` | `npc/tests.rs:1977` left `0` right `250` |
| 10 | `npc::tests::create_non_cumulative_spawns_n_items` | `npc/tests.rs:1925` left `0` right `5` (blank runes) |
| 11 | `npc::tests::create_respects_data_rune_charges` | `npc/tests.rs:2371` left `0` right `1` |
| 12 | `npc::tests::create_respects_data_subtype` | `npc/tests.rs:2314` vial fluid left `0` right `11` |
| 13 | `npc::tests::money_create_delete_and_insufficient` | `npc/tests.rs:555` left `0` right `250` |
| 14 | `npc::tests::partial_failure_keeps_prior_mutations` | `npc/tests.rs:871` gold left `0` right `40` |
| 15 | `npc::tests::vial_deposit_no_money_duplication` | `npc/tests.rs:2054` left `0` right `5` |
| 16 | `shop::tests::buy_with_money_adds_items` | `shop.rs:678` — no extra backpack after purchase |
| 17 | `shop::tests::remove_gold_works_while_shop_open` | `shop.rs:693` `player_remove_item_of_type` gold failed |
| 18 | `shop::tests::sale_list_counts_refresh_on_inventory_change` | `shop.rs:748` left `None` right `Some(4)` |
| 19 | `shop::tests::sell_removes_items_and_pays` | `shop.rs:717` `bags_before >= 2` |

---

## Death / loot identity (4)

| # | Test | Panic |
|---|------|--------|
| 20 | `death::tests::m6_death_penalty_flat_772_promoted_7_percent` | `death.rs:577` exp left `9000` (10%) right `9300` (7%) — promotion not applied |
| 21 | `death::tests::m6_death_penalty_flat_772_promoted_with_all_blessings_zero` | `death.rs:617` exp left `9500` right `9800` |
| 22 | `data_pack_lua_tests::rarity_survives_death_v772` | `data_pack_lua_tests.rs:334` living-monster ACTIONID left `0` right `4242` |
| 23 | `game_world_inventory::add_health_tests::add_health_on_monster_kills_at_zero` | `game_world_inventory.rs:2338` monster at 0 HP not removed |

---

## Vocations path / config (4)

`player::active_vocation` loads `data/defs/vocations.lua` relative to CWD. `cargo test -p tfs-rust-core` uses package dir `crates/tfs-rust-core`, so that path misses repo-root `data/`.

| # | Test | Panic |
|---|------|--------|
| 24 | `player::active_vocation::tests::config_defaults_premium_promotion_true` | `active_vocation.rs:191` `premium_promotion_enabled(&cfg)` |
| 25 | `player::active_vocation::tests::free_account_uses_effective_vocation_when_enabled` | `active_vocation.rs:149` `vocations.lua` ENOENT |
| 26 | `player::active_vocation::tests::premium_account_keeps_stored_vocation_when_enabled` | same ENOENT |
| 27 | `player::active_vocation::tests::toggle_off_lets_free_accounts_stay_promoted` | same ENOENT |

---

## One-offs (2)

| # | Test | Panic |
|---|------|--------|
| 28 | `game_world_item_cylinder::item_move_event_tests::move_item_onto_aid_tile_with_no_actor_fires_on_item_move` | `game_world_item_cylinder.rs:1158` expected RemoveItem on source; got Add (`is_add: true`) at `[61,60,7]` |
| 29 | `monster_ai::world_tests::weakest_opponent_metric_follows_profile` | `monster_ai_world_tests.rs:661` picked `CreatureId(1v1)` expected `CreatureId(2v1)` |
