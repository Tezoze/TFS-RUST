# Field step-in battle sign — 2026-08-16

Walking on poison/fire/energy fields does not show the swords icon. 772 `TPlayer::DamageStimulus` (`crplayer.cc:382-385`) always `BlockLogout(60, false)` for a living player, including field collision with `Attacker == NULL` (`moveuse.dat` `Damage(Obj1,Obj2,32,100)`). Rust gated victim infight on `attacker: Some`.

## Work

- [x] Call victim `DamageStimulus` (Infight) from periodic arm even when attacker is None
- [x] Same on the HP path for field `initdamage` (fire 20 / energy 30)
- [x] Test: poison field under player applies `CONDITION_INFIGHT`
- [x] Lesson
