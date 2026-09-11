# Data-pack Lua vs native corpus

TFS `data/` scripts are the **pack surface** (spell names, `Condition` userdata, `CONDITION_PARAM_TICKS`). Core outcomes follow the 772 decompile via `MechanicsProfile` / `data/formulas/772.lua`.

## Skill timers (haste, strong haste, paralyze, magic shield, invisibility, light)

Pack scripts still set `CONDITION_PARAM_TICKS` (haste 30000, light 370000, …). Native `active_condition_from_apply_spec` **ignores** those durations for the types above and arms `SetTimer(Cycle, Count, MaxCount)` from `formulas.skillTimers`:

| Spell | Cycle, Count, MaxCount | Observable |
|---|---|---|
| Haste | 3, 10, 10 | 33 s speed |
| Strong haste (`speed >= 60`) | 2, 10, 10 | 22 s |
| Paralyze | 1, 10, 10 | 11 s |
| Magic shield | 1, 200, 200 | icon off at 202 s |
| Invisibility | 1, 200, 200 | outfit back at 201 s |
| Light | radius, Duration/radius, Duration/radius | utevo lux 504 s; radius shrinks each Event |

Pack `great_light.lua` light level 7 is treated as corpus radius 8. Do not “fix” the Lua ticks to match — corpus numbers live in `772.lua`.

## House rent cadence

Corpus `ProcessHouses` runs once at boot (`InitHouses`) and on the daily save/reboot fire. Native `process_and_persist_houses_inner` matches that — not the TFS per-minute scan. Pack `housePriceRentPeriod` still drives **paid_until** math inside `decide_rent` (daily/weekly/never), not how often the scan runs.

## Raid XML `radius` vs corpus `Wave->Radius`

TFS `data/raids/*.xml` `<areaspawn radius>` is spawn **spread** (random offset). Corpus `.evt` has both `spread` (offset) and `radius` (per-creature `TCreature::Radius` leash). Pack XML has no leash attribute — `Monster::radius` stays `i32::MAX`. Do not map XML radius onto the leash.

## Daily save notify duration

Pack `config.lua` `serverSaveNotifyDuration` is kept for TFS compatibility but **unused**. Native warnings fire at fire−5 / −3 / −1 minutes (`main.cc:399-422`), not a single TFS duration. Reboot wording when `serverSaveShutdown == false`; going-down otherwise.
