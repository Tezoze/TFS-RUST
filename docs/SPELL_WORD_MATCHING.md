# Spell Word Matching — 772 vs TFS/1098

## How 772 decompile works

### Syllable-based parsing (`magic.cc`)

772 stores spells as **syllable arrays**, not comma-separated strings.

**`SpellSyllable[51][6]`** (`magic.cc:33-85`) — a lookup table of 51 syllables:
```
"", "al", "ad", "ex", "ut", "om", "para", "ana", "evo", "ori", "mort",
"lux", "liber", "vita", "flam", "pox", "hur", "moe", "ani", "ina",
"eta", "amo", "hora", "gran", "cogni", "res", "mas", "vis", "som",
"aqua", "frigo", "tera", "ura", "sio", "grav", "ito", "pan", "vid",
"isa", "iva", "con", ...
```

**`SpellList[256]`** (`magic.cc:30`) — each entry stores `uint8 Syllable[MAX_SPELL_SYLLABLES]`
(indices into `SpellSyllable`), not strings. Spells are registered via:
```c
CreateSpell(1, "ex", "ura", "");        // Light Healing
CreateSpell(2, "ex", "ura", "gran", ""); // Intense Healing
CreateSpell(3, "ex", "ura", "vita", ""); // Ultimate Healing
CreateSpell(24, "ex", "evo", "gran", "mas", "vis", ""); // UE
```

### Text parsing (`CheckForSpell` — `magic.cc:3913-3975`)

When a player says text, the server parses it **syllable by syllable**:

1. **`TypeOfSpell(Text)`** (`magic.cc:3702-3718`) — checks the first 2 chars
   against syllables 1–5 (`"al"`, `"ad"`, `"ex"`, `"ut"`, `"om"`). This determines
   the spell type (character right, rune, cast, account right). If no match →
   not a spell.

2. **Tokenize by spaces** — `std::istringstream IS(Text)`, skip first 2 chars
   (the type syllable), then read space-separated tokens:
   ```c
   IS.get(); IS.get();  // consume first syllable
   while (!IS.eof()) {
       while (isSpace(IS.peek())) IS.get();  // skip whitespace
       if (IS.peek() == '"') {
           IS.get();
           IS.get(SpellStr[Index], sizeof, '"');  // quoted parameter
           IS.get();
       } else {
           IS.get(SpellStr[Index], sizeof, ' ');  // space-delimited token
       }
   }
   ```

3. **Match each token to a syllable** — `stricmp(token, SpellSyllable[n])`.
   If no match → it's a **parameter** (stored as syllable index 6 = `"para"`).

4. **`FindSpell(Syllable[])`** (`magic.cc:3720-3752`) — iterates all 256 spell
   entries, finds the best match (fewest unmatched parameters).

### Key 772 behaviors
- **Spaces are the delimiter** between syllables: `"ex evo gran mas vis"` (UE)
- **First 2 syllables have no space** between them: `"exura"` not `"ex ura"`
  (`GetSpellString` `magic.cc:3797`: `if (i >= 2) strcat(Text, " ")`)
- **Quoted strings** (`"..."`) are spell parameters (e.g. teleport target name)
- **Case-insensitive** (`stricmp`)
- **Extra tokens become parameters** — syllable index 6 (`"para"`) marks a
  parameter slot in the spell definition

### 772 spell string format
`GetSpellString` (`magic.cc:3780-3803`) reconstructs the spell string as:
- Syllables 0 and 1 concatenated with no space: `"ex" + "ura"` = `"exura"`
- Syllables 2+ separated by spaces: `"exura gran"` (Intense Healing)
- Full UE: `"exevo gran mas vis"` (no commas, spaces between syllables 2+)

---

## How TFS/1098 works

### Comma-separated words (`tools.cpp`)

TFS stores spell words as **comma-separated strings**: `"ex,ori"`, `"ex,evo, vis, lux"`.

**`compareSpellWords(spellWords, givenWords, supportParam)`** (`tools.cpp:386-411`):
1. Split `spellWords` by `,` into `spellVector`
2. For each syllable in `spellVector`:
   - Strip leading space from `givenWords` if the spell syllable doesn't start with space
   - Compare the first N chars of `givenWords` against the syllable (case-insensitive)
   - Advance `givenWords` past the matched syllable
3. If `givenWords` has remaining text and `!supportParam` → no match
4. If all syllables matched → match

**`mergeSpellWords(words)`** (`tools.cpp:413-421`) — removes commas, concatenates:
`"ex,evo, vis, lux"` → `"exevo vis lux"`

**`countSpaces(str)`** (`tools.cpp:375-384`) — counts whitespace chars.

### `playerSaySpell` (`spells.cpp:30-69`)
1. `trimString` + `removeExtraSpaces` on the input text
2. `getInstantSpell(words)` — iterates all instant spells, calls `compareSpellWords`
3. Extracts the parameter (text after the spell words)
4. Handles quoted parameters (`"..."`)

### Key TFS behaviors
- **Commas separate syllables** in the registered words: `"ex,evo, vis, lux"`
- **Spaces in the registered words** are preserved and matched against the input
- `"ex,evo, vis, lux"` means: match `"exevo"` then `" vis"` then `" lux"`
  (the space before `vis` and `lux` is part of the syllable)
- The player types: `"exevo vis lux"` (spaces between all syllables after the first)
- **Case-insensitive** (`boost::iequals`)
- **Parameters** — text after the spell words; quoted with `"`

### The comma+space convention
The Lua scripts register words like `"ex,evo, vis, lux"` where:
- `ex` — no leading space (first syllable, glued to second)
- `evo` — no leading space (second syllable, glued to first: `"exevo"`)
- ` vis` — leading space (third syllable, separated by space: `"exevo vis"`)
- ` lux` — leading space (fourth syllable: `"exevo vis lux"`)

This mirrors the 772 `GetSpellString` rule: syllables 0+1 are concatenated
without space, syllables 2+ are separated by spaces. The commas in the Lua
format are just delimiters — `compareSpellWords` splits on them and matches
each piece (including leading spaces) against the input text.

---

## Our current Rust implementation

**`SpellRegistry::get_instant_by_words`** (`spells.rs:134`):
```rust
pub fn get_instant_by_words(&self, words: &str) -> Option<&InstantSpellDef> {
    self.instant_by_words.get(&words.to_ascii_lowercase())
}
```

This does an **exact match** on the full words string (lowercased). It does NOT:
- Split by commas
- Handle spaces between syllables
- Match `"exevo vis lux"` against the registered `"ex,evo, vis, lux"`
- Support parameters (text after the spell words)

**`player_say_spell`** (`game_world_chat.rs:213`):
```rust
let Some(spell) = self.spells.get_instant_by_words(text).cloned() else {
    return false;
};
```

This passes the **raw chat text** (e.g. `"exevo vis lux"`) to `get_instant_by_words`,
which looks up `"exevo vis lux"` in a HashMap keyed by `"ex,evo, vis, lux"` — **no match**.

---

## What needs to change

### Option A: Implement `compareSpellWords` (TFS approach)

Port `compareSpellWords` + `mergeSpellWords` + `countSpaces` from `tools.cpp`:

1. **`compare_spell_words(spell_words: &str, given: &str, support_param: bool) -> bool`**
   - Split `spell_words` by `,`
   - For each syllable, match the first N chars of `given` (case-insensitive)
   - Strip leading space from `given` if the syllable doesn't start with space
   - If remaining text and `!support_param` → false

2. **`get_instant_spell(text: &str) -> Option<&InstantSpellDef>`**
   - Trim + remove extra spaces from `text`
   - Handle quoted parameters (strip text after `"`)
   - Iterate all instant spells, call `compare_spell_words`
   - Return first match

3. **`player_say_spell`** — replace `get_instant_by_words(text)` with
   `get_instant_spell(text)` (iterative match, not HashMap lookup)

### Option B: Pre-compute merged words as HashMap key

At registration time, compute `mergeSpellWords("ex,evo, vis, lux")` → `"exevo vis lux"`
and store in a second HashMap keyed by the merged form. Then `get_instant_by_words`
can do a direct lookup on the trimmed input text.

**Limitation:** doesn't handle parameters (text after the spell words). For
parameter spells (find_person `"ex,iva"`, teleport `"ad,ana, vita"`), the input
text has extra content after the spell words.

### Recommended: Option A (full `compareSpellWords` port)

This is the correct approach — it handles:
- Comma-separated registered words with spaces
- Parameters (quoted and unquoted)
- Case-insensitive matching
- The 772 "first two syllables glued" convention

The HashMap lookup can remain as a fast-path for exact matches (no parameters),
with the iterative `compareSpellWords` as the fallback for parameter spells.

---

## Test cases

| Registered words | Player types | Match? | Notes |
|-----------------|-------------|--------|-------|
| `"ex,ori"` | `"exori"` | ✅ | Berserk — 2 syllables, no space |
| `"ex,ori"` | `"exori "` | ✅ | Trailing space trimmed |
| `"ex,ori"` | `"EXORI"` | ✅ | Case-insensitive |
| `"ex,evo, vis, lux"` | `"exevo vis lux"` | ✅ | Energy beam — spaces after syllable 2+ |
| `"ex,evo, vis, lux"` | `"exevo  vis  lux"` | ✅ | Extra spaces (removeExtraSpaces) |
| `"ex,ura"` | `"exura"` | ✅ | Light healing |
| `"ex,ura, gran"` | `"exura gran"` | ✅ | Intense healing |
| `"ex,iva"` | `"exiva PlayerName"` | ✅ | Find person (with param) |
| `"ex,iva"` | `"exiva"` | ✅ | Find person (no param — still matches) |
| `"ex,ori"` | `"exori blah"` | ❌ | Extra text, no param support |
| `"ad,ana, vita"` | `"adana vita"` | ✅ | Ultimate healing rune words |
