# Native Player:conjureItem + formulas.conjureFromHandsOnly

**Status:** complete.

- `MechanicsProfile.conjure_from_hands_only` from `formulas.conjureFromHandsOnly` (772/1098 default **true**).
- Native `creature:conjureItem` in `conjure.rs`; first arg integer mana or Spell userdata (`spell.mana`).
- Lua `Player:conjureItem` removed; spell scripts unchanged.
