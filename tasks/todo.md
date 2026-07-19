# PZ lock on monster SetAttackDest (2026-07-19)

- [x] Align to 772: `Target` assign ≠ lock; idle walk `SetAttackDest` → `AttackStimulus`
- [x] `selectTarget` / summon rebind: follow only (no early `attack_target`)
- [x] `monster_idle_maybe_enter_attacking`: fist>0 → ATTACKING, then SetAttackDest (no melee gate)
- [x] Lose-target clears `Target` only (not AttackDest); tests + lesson #226
