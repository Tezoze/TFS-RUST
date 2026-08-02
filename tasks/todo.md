# P0: PvP damage half + kill experience — 2026-08-02

From player-pvp-audit canvas. 772 outcomes; TFS-shaped domain.

- [x] `(Damage+1)/2` in `combat_execute_with_stimulus` before absorb (player↔player, non-DoT)
- [x] Rewrite `pvp_exp_cap` → MaxLevel scale; death pool `Exp/20` only on `PvpEnforced` + party skip
- [x] Wire `WorldType` into `handle_creature_death`
- [x] Tests + lessons + canvas P0 Done
