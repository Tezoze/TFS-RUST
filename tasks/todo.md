# PC-3a PvE combat polish (skip PvP) — 2026-07-20

- [x] DistanceAttack: full attack − armor only; GetDefendDamage SE when attacker has shield
- [x] Burst arrow via Lua `onUseWeapon` → `Combat:execute` (`burst_arrow.lua`) — not hardcoded AoE
- [x] Wire Lua breakChance/action + Fragility Delete vs Move-to-drop
- [x] aoe.rs: apply block_shield + block_armor for Physical Combat:execute
- [x] Resolve COMBAT_FORMULA_SKILL via MechanicsProfile (772 ClassicProbe / 1098 TFS 0.085)
- [x] Destroy chargeable weapon/shield when wearout would hit count 0
- [x] Regression tests + lessons/plan residuals
- [x] Production 772 combat ProbeValue/armor/hit → `world.parity_rng` (glibc), not `ai_rng`
- [x] Unify RNG: remove `ai_rng` / `ParityRngSource`; both eras use per-world glibc
- [x] Monster `GetArmorStrength` / `GetDefendValue`: race + equipped body/hands via live snapshot

**Out of scope (still deferred):** PvP skulls / RecordAttack / protectionLevel / world-type gates.
