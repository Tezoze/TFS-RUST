# NPC corpus inventory (NPC-0)

Authority: `data/npc/behavior`

## Coverage

- `.npc` files: **337**
- `.ndb` fragments: **39**
- Include directives (`@"…"`): **165** (unique targets: 22)
- Reference cross-check (`reference/cipsoft-772/runtime/npc`): 337 npc / 39 ndb

## Encodings

- utf-8: 372
- non-utf8: brodrosch.npc, captain bluebear.npc, captain seahorse.npc, uzon.npc
- newlines: {'lf': 376, 'crlf': 0, 'none': 0}

## Substitutions

- `%1`: 7477
- `%A`: 1740
- `%N`: 657
- `%P`: 4650
- `%T`: 110

## Special tokens

- `!`: 3193
- `$`: 2359
- `%1`: 7461
- `*`: 5372
- `->`: 20174
- `@include`: 165

## Functions (call sites)

- `bless`: 7
- `burning`: 158
- `count`: 230
- `create`: 245
- `delete`: 149
- `effectme`: 170
- `effectopp`: 542
- `poison`: 41
- `profession`: 9
- `questvalue`: 891
- `random`: 7
- `setquestvalue`: 249
- `spellknown`: 34
- `spelllevel`: 68
- `summon`: 19
- `teachspell`: 34
- `teleport`: 103
- `town`: 9

## Assignments

- `amount`: 4104
- `behavior`: 337
- `data`: 247
- `hp`: 79
- `price`: 4714
- `string`: 595
- `topic`: 8080
- `type`: 4011

## Properties seen

- `address`: 1313
- `busy`: 1168
- `druid`: 362
- `female`: 93
- `knight`: 80
- `male`: 104
- `paladin`: 171
- `premium`: 100
- `promoted`: 12
- `pvpenforced`: 199
- `pzblock`: 47
- `sorcerer`: 352
- `vanish`: 327

## Unsupported / ambiguous constructs

- **function_call** `bless` (raw: Bless; n=7): not in 772 action/expression tables (crnonpl.cc)
- **function_call** `town` (raw: Town; n=9): not in 772 action/expression tables (crnonpl.cc)
- **assignment** `string` (raw: String; n=595): not in 772 SET_VARIABLE / SET_SKILL action ids
- **unknown_action** `promote` (raw: Promote; n=4): not in 772 action/property tables (crnonpl.cc)
- **encoding** `brodrosch.npc` (raw: —; n=1): file is non-utf8 (latin-1 decode used for inventory)
- **encoding** `captain bluebear.npc` (raw: —; n=1): file is non-utf8 (latin-1 decode used for inventory)
- **encoding** `captain seahorse.npc` (raw: —; n=1): file is non-utf8 (latin-1 decode used for inventory)
- **encoding** `uzon.npc` (raw: —; n=1): file is non-utf8 (latin-1 decode used for inventory)

## Case variants (supported after lowercase)

- `amount`: Amount, amount
- `druid`: Druid, druid
- `idle`: Idle, idle
- `knight`: Knight, knight
- `level`: Level, level
- `paladin`: Paladin, paladin
- `premium`: Premium, premium
- `promoted`: Promoted, promoted
- `questvalue`: QuestValue, Questvalue
- `setquestvalue`: SetQuestValue, SetQuestvalue
- `sorcerer`: Sorcerer, sorcerer
- `topic`: Topic, topic
