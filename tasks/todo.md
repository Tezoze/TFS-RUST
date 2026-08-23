# Active plans

- **Monsters XML → Lua:** [monsters-lua-plan.md](monsters-lua-plan.md) (Lua-as-data, not TFS `createMonsterType`).
  - Converter **done** (`export-monsters-lua`); 157 `data/monster/*.lua`; XML kept; runtime still XML.
  - Immunity pass **done** (all 8 bits always emitted; `NoParalyze`; life-drain copy). Lesson 365.
  - **One corpus** for 772 and 1098; era gates are combat types/conditions (death/holy/earth/ice), not a second attack shape.
  - Remaining: switch `load_dir` to Lua, then delete XML.

# Monster combat 772 parity — audit fixes

**Status:** done except deferred tie-break. Lesson 363.

Shipped: parse `targetstrategy` + `losetarget`; 90 XML `LoseTarget` ports; PZ / cross-floor / cheb>8 `StopAttack`; acquire Z = `spectator_z_range`; player-master skip; summon empty-target `ToDoWait(1000)`; fist `Increase(1)`; poison clamp; fire/energy no `.max(1)`; variation==0 skip rand.

**Deferred:** spectator scanline vs C++ creature-chain insertion order (strategy ties).
