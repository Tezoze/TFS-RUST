# NPC-0 black-box transcript fixtures

These JSON files freeze **expected ordered events** for representative legacy NPC
behaviors. They are authored from 772 C++ semantics in
`reference/cipsoft-772/tibia-game-master/src/crnonpl.cc` (`TalkStimulus`,
`IdleStimulus`, `TBehaviourDatabase::react`) — not from a live C++ harness.

## Layout

| File | Focus |
|------|--------|
| `greeting_farewell.json` | ADDRESS → DEFAULT → Idle |
| `quentin_heal.json` | Heal branches + EffectOpp |
| `zebron_gamble.json` | `%1` capture, Random, money |
| `bank_change.json` | `gen-bank.ndb` change-making |
| `explorer_quest.json` | QuestValue / SetQuestValue |
| `guard_thais.json` | Shared `guards-thais.ndb` |
| `multi_reply_timing.json` | Multi-`REPLY` delay chain |
| `two_player_busy_queue_vanish.json` | BUSY/Queue/timeout/ADDRESSQUEUE |

## Authoring rules

1. Cite `cpp_refs` for non-obvious outcomes.
2. List every `sources` file the scenario depends on (behavior + includes).
3. Compute `say.delay_ms` with the formula in [`schema.md`](schema.md).
4. When `Random(...)` appears, record `declared_rng` until a seeded C++ capture exists.
5. Validate with:

```bash
python3 scripts/validate_npc_fixtures.py
```

## Downstream use

- **NPC-4:** pure dialogue/state events must match these fixtures.
- **NPC-5/6:** mutating and timing events extend the same files or siblings.
