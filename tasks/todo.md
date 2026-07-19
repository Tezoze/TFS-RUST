# Game-loop performance audit — batch checklist

See `docs/GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md` § Next steps (full detail).
Landed remediation: `68b7c93`.
OBS-1 full histograms: this pass (Phase 0 code).

## Done (Phases 1–4 code + post-landing regressions)
- [x] GL-1 async login
- [x] GL-2 dual-lane command ingress + budget
- [x] GL-3 bounded outbound + sink map
- [x] DEC-1…DEC-4 decay apply / heap / clock / depot cache
- [x] IDLE-1 / IDLE-2 spells + path scratch
- [x] TODO-1 iterative Execute (+ guard re-arm)
- [x] IDLE-3 / GL-4 / OBS-1 thin follow-ups
- [x] Session/path/decay regressions (logout TCP, LocalSet saves, lane-full shed, expand_next, stop_decay/look-save ms, mid-login disconnect)

## Next — 0. Manual smoke
- [ ] Floor change onto dense monsters (no desync / no wedge)
- [ ] Logout returns to character list immediately
- [ ] Ctrl+C exits cleanly (≤10s / 2nd Ctrl+C)
- [ ] Failed login / logout mid-load closes TCP

## Next — 1. Phase 0 + full OBS-1
- [x] Aggregated histograms (loop / output / subsystems / ToDo / idle-path / decay)
- [ ] 60s baselines: idle, dense spawn, chase, spell fight, corpse wave, packet flood (p50/p95/p99) — template: `docs/GAME_LOOP_OBS_BASELINES.md`

## Next — 2. Close partial findings
- [ ] IDLE-3 sector-order / generation dedup where needed
- [ ] GL-4 active indexes only if load data requires it
- [ ] Beat-startup parity (`interval_at`) if unintended
- [ ] (Low) failed-login `disconnectClient` error string

## Next — 3. Full-scale required tests
- [ ] #1–3, #5, #7–8, #10 (see audit doc table)

## Next — 4. Load-test gate
- [ ] Load-test optimized path
- [ ] TODO-2 overload budget — only if still needed after load-test
