# M4 — 772 trap / field script pass

**Status:** done. Lesson 362.

Peaceful fields 1500/1501/1503/1504 use `field.skippeaceful` + `creature_is_peaceful` (poff, no init/DoT). Searing 1506/1507 native `initdamage=300`/`cycles=10`; `fields.lua` no-op. `trap.lua` Collision: 1510 transform-only, 1511/1513 60 physical, no PZ skip, bear `dontDamagePlayers`.
