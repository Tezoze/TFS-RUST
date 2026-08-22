# Monster combat 772 parity — audit fixes

**Status:** done except deferred tie-break. Lesson 363.

Shipped: parse `targetstrategy` + `losetarget`; 90 XML `LoseTarget` ports; PZ / cross-floor / cheb>8 `StopAttack`; acquire Z = `spectator_z_range`; player-master skip; summon empty-target `ToDoWait(1000)`; fist `Increase(1)`; poison clamp; fire/energy no `.max(1)`; variation==0 skip rand.

**Deferred:** spectator scanline vs C++ creature-chain insertion order (strategy ties).
