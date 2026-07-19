# Game-loop performance audit — batch checklist

See `docs/GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md` § Next steps (full detail).
Landed remediation: `68b7c93`.
Follow-up (IDLE-3 / beat-startup / full-scale tests / OBS-1): landed this commit.

## Done (Phases 1–4 + post-landing + engineering follow-up)
- [x] GL-1 async login
- [x] GL-2 dual-lane command ingress + budget
- [x] GL-3 bounded outbound + sink map
- [x] DEC-1…DEC-4 decay apply / heap / clock / depot cache
- [x] IDLE-1 / IDLE-2 spells + path scratch
- [x] TODO-1 iterative Execute (+ guard re-arm)
- [x] IDLE-3 sector-order + gen-marked dedup
- [x] Beat-startup `interval_at` parity
- [x] Full-scale tests #1–3, #5, #7–8, #10
- [x] OBS-1 aggregated histograms (`tfs_obs` — opt-in only)
- [x] Session/path/decay regressions (logout TCP, LocalSet saves, lane-full shed, expand_next, stop_decay/look-save ms, mid-login disconnect)

## Next — 0. Manual smoke (operator)
Day-to-day: `RUST_LOG=warn` or `info,tfs_obs=off` (do not leave `tfs_obs=info` on).
- [ ] Floor change onto dense monsters (no desync / no wedge)
- [ ] Logout returns to character list immediately
- [ ] Ctrl+C exits cleanly (≤10s / 2nd Ctrl+C)
- [ ] Failed login / logout mid-load closes TCP

## Next — 1. Live OBS baselines (operator, opt-in)
- [x] Aggregated histograms (loop / output / subsystems / ToDo / idle-path / decay)
- [ ] 60s baselines: idle, dense spawn, chase, spell fight, corpse wave, packet flood — `docs/GAME_LOOP_OBS_BASELINES.md`
  - Capture with: `RUST_LOG=tfs_obs=info,warn ./scripts/run_server.sh`

## Next — 2. Deferred until data
- [ ] GL-4 active indexes only if load data requires it
- [ ] (Low) failed-login `disconnectClient` error string

## Next — 3. Load-test gate
- [ ] Load-test optimized path
- [ ] TODO-2 overload budget — only if still needed after load-test
