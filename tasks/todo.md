# Player combat PvE polish — 2026-07-22

**Scope:** everything but PvP. Canvas: `player-combat-audit.canvas.tsx`.

## Closed

- [x] DistanceAttack defense (attacker-shield SE only; attack − armor)
- [x] Burst via Lua `onUseWeapon` (`burst_arrow.lua`)
- [x] Poison via Lua `onUseWeapon` (`poison_arrow.lua`)
- [x] Fragility / breakChance Delete vs Move-to-drop
- [x] aoe.rs `block_shield` + `block_armor` for Physical
- [x] `COMBAT_FORMULA_SKILL` (772 one ProbeValue → `(v,v)`)
- [x] Wearout destroy on last charge
- [x] Distance hit: Probe + GetAttackDamage → two `Increase(1)` + two LP--
- [x] Wand/rod Lua level + vocation gates (`WandDef` only — not melee weapons)
- [x] Equip hand/ammo → `DelayAttack(2000)` (`CheckCombatValues`)

## Still deferred

- [ ] Delayed `StopAttack` / `LatestAttackTime` (low; rare in PvE)
- [ ] PvP skulls / RecordAttack / protectionLevel / world-type gates
