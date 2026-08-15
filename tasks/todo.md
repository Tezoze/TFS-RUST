# E8–E9: drunk stack + `getFormattedWorldTime` — 2026-08-15

From [other-actions-plan.md](other-actions-plan.md). **Done.**

## E8 Drunk stack + `fluids.lua`
- [x] Beer/wine: if Cycle `< 5` then `+1`; Count=MaxCount=120; `base.drunkenness` = Cycle
- [x] `ProcessSkills`: Count--; on 0, Cycle toward 0, Count=MaxCount; at 0 remove
- [x] Spell-drunk stays Power-gated; uses Duration as MaxCount
- [x] Rewrite `fluids.lua` to `UseLiquidContainer`

## E9 `getFormattedWorldTime`
- [x] Inject from `data/global.lua` (no full `dofile`)
- [x] `watch.lua`: pendulum 1728–1731, watch 2036, cuckoo 1873–1877 and 1881; drop 3900
