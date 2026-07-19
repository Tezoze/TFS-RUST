# Game-loop performance audit — batch checklist

## Done (Phases 1–4 code)
- [x] GL-1 async login
- [x] GL-2 dual-lane command ingress + budget
- [x] GL-3 bounded outbound + sink map
- [x] DEC-1…DEC-4 decay apply / heap / clock / depot cache
- [x] IDLE-1 / IDLE-2 spells + path scratch
- [x] TODO-1 iterative Execute
- [x] IDLE-3 / GL-4 / OBS-1 thin follow-ups

## Live bugfix (floor-change desync with monsters)
- [x] GL-3 outbound soft-cap shed on empty-queue `0x64` bursts
- [x] IDLE-2 `TShortwayScratch`: reset `expand_next`/`parent` on gen change (wake/target storm)
- [x] Ctrl+C hang: LocalSet-safe flush + 10s / 2nd Ctrl+C force-exit
- [x] Logout: drop registry `OutboundTx` so TCP closes (TFS `disconnect`)
- [x] Review fixes: outbound re-queue, GameLaneFull shed, onLogout cancel, disconnect save
- [x] Review #2: mid-login logout, failed-login TCP close, stop_decay item-ms return
- [x] Review #3: todo execute guard re-arm; look/save `item_decay_remaining_ms`
- [ ] Retest: floor change onto dense monsters after server restart
- [ ] Retest: client logout returns to character list cleanly

## Next (see `docs/GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md` § Next steps)
- [ ] Phase 0 + full OBS-1 histograms and 60s baselines (p50/p95/p99)
- [ ] IDLE-3 sector-order / generation dedup where needed
- [ ] GL-4 active indexes only if load data requires it
- [ ] Beat-startup parity (`interval_at`) if unintended
- [ ] Full-scale required tests #1–3, #5, #7–8, #10
- [ ] Load-test optimized path
- [ ] TODO-2 overload budget — only if still needed after load-test
