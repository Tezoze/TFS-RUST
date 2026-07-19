# Game-loop performance audit — batch checklist

See `docs/GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md` § Next steps (full detail).
Landed remediation: `68b7c93`.
Follow-up (IDLE-3 / beat-startup / full-scale tests / OBS-1): landed.
MAP-walk open-container fix + live decay/chase verification: 2026-07-19.

## Done
- [x] GL-1…GL-3, DEC-1…DEC-4, IDLE-1…IDLE-3, TODO-1, OBS-1
- [x] Beat-startup `interval_at` + full-scale tests #1–3, #5, #7–8, #10
- [x] Session/path/decay regressions
- [x] Live chase smoke (~20 monsters lure/kite)
- [x] Live corpse decay (in-game stages + OBS `decay_due`; cheap cron)
- [x] MAP-walk: open ground corpse/container + walk → O(1) `script_item_position` (verified)

## Next — optional / deferred
- [ ] Fill remaining baseline rows (idle dense / spell / packet flood) in `docs/GAME_LOOP_OBS_BASELINES.md`
- [ ] Harder multi-client / denser load-test
- [ ] GL-4 active indexes — only if load data requires
- [ ] (Low) failed-login `disconnectClient` error string
- [ ] TODO-2 overload budget — only if still needed after harder load-test

Day-to-day: default mutes `tfs_obs`; opt-in with `RUST_LOG=info,tfs_obs=info`.
