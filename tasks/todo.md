# Monster combat audit — 2026-07-22

**Scope:** Open Bugs/Gaps/Partials from monster-combat-audit canvas. TFS domain + 772 outcomes. Skip IMPACT_STRENGTH.

## Plan

- [x] B1 — Extract physical mitigate helper; wire CASTING Damage Physical + reuse from aoe.rs
- [x] Parse poison→Earth, manadrain→ManaDrain, knife/rock/stone shooteffects
- [x] B2 — Speed MDAct% + Haste/Paralyze + duration rounds
- [x] B3 — Drunk Power=drunkness/20≤6 + duration timer
- [x] Outfit + Invisible SpellImpact + ConditionOutfit look_type_ex + ProcessSkills
- [x] Fist-only Attack distance=1; cast target = follow_target only
- [x] Tests + lessons / todo

## Deferred

- [ ] IMPACT_STRENGTH (no TFS XML name=strength)
