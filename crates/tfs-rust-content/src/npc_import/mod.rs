//! Offline one-way importer: legacy `.npc`/`.ndb` (+ optional XML) → Lua definitions.
//!
//! Not a runtime loader. AST types stay private; public API lowers into
//! [`crate::npcs::PendingNpcDefinition`] and emits deterministic Lua.
//!
//! Domain: TFS `data/npc/scripts/` authoring.
//! 772 outcomes: `reference/cipsoft-772/runtime/npc` full files (`Name`/`Behaviour`),
//! `crnonpl.cc` condition/action tables.

mod ast;
mod decode;
mod emit;
mod error;
mod include;
mod lex;
mod lower;
mod parse;
mod xml_meta;

pub use emit::{definition_filename, emit_npc_lua};
pub use error::{ImportError, ImportResult};
pub use lower::lower_npc;
pub use parse::{parse_ndb_rules, parse_npc_file, parse_npc_source};
pub use xml_meta::{apply_xml_meta, load_xml_npc_dir, XmlNpcMeta};

use std::path::{Path, PathBuf};

use crate::npcs::PendingNpcDefinition;

/// Import all full `.npc` files under `root` (skips `.ndb`; those are includes only).
pub fn import_legacy_root(root: &Path) -> ImportResult<Vec<PendingNpcDefinition>> {
    let root = root
        .canonicalize()
        .map_err(|e| ImportError::io(root, e.to_string()))?;
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&root)
        .map_err(|e| ImportError::io(&root, e.to_string()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("npc"))
        .collect();
    paths.sort();

    let mut out = Vec::with_capacity(paths.len());
    let mut errors = Vec::new();
    for path in paths {
        match parse_npc_file(&root, &path).and_then(lower_npc) {
            Ok(pending) => out.push(pending),
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }
    if !errors.is_empty() {
        return Err(ImportError::msg(format!(
            "import failed for {} file(s):\n{}",
            errors.len(),
            errors.join("\n")
        )));
    }
    out.sort_by_key(|a| a.name.to_ascii_lowercase());
    Ok(out)
}

/// Split mode: `data/npc/*.xml` with `behavior=` → load behavior from `behavior_dir`.
pub fn import_split_xml(
    xml_dir: &Path,
    behavior_dir: &Path,
) -> ImportResult<Vec<PendingNpcDefinition>> {
    let behavior_root = behavior_dir
        .canonicalize()
        .map_err(|e| ImportError::io(behavior_dir, e.to_string()))?;
    let metas = load_xml_npc_dir(xml_dir)?;
    let mut out = Vec::new();
    let mut errors = Vec::new();

    for meta in metas {
        if meta.script.is_some() && meta.behavior.is_none() {
            // Compatibility script NPCs — skip in importer (NPC-7 migrates them).
            continue;
        }
        let Some(beh) = meta.behavior.as_ref() else {
            errors.push(format!(
                "{}: xml has neither behavior= nor script=",
                meta.name
            ));
            continue;
        };
        let path = behavior_root.join(beh);
        // Behavior-only files wrap rules in Behavior = { }; synthesize a full file
        // by prepending Name metadata from XML.
        match import_behavior_with_xml_meta(&behavior_root, &path, &meta) {
            Ok(p) => out.push(p),
            Err(e) => errors.push(format!("{} ({}): {e}", meta.name, path.display())),
        }
    }

    if !errors.is_empty() {
        return Err(ImportError::msg(format!(
            "split import failed for {} NPC(s):\n{}",
            errors.len(),
            errors.join("\n")
        )));
    }
    out.sort_by_key(|a| a.name.to_ascii_lowercase());
    Ok(out)
}

fn import_behavior_with_xml_meta(
    root: &Path,
    behavior_path: &Path,
    meta: &XmlNpcMeta,
) -> ImportResult<PendingNpcDefinition> {
    let raw = include::read_npc_file(behavior_path)?;
    // If file is behavior-only, wrap with Name + Behaviour for the full-file parser.
    let needs_wrap = !raw.to_ascii_lowercase().contains("name");
    let synthetic = if needs_wrap {
        let body = raw.trim();
        let body = body
            .strip_prefix("Behavior")
            .or_else(|| body.strip_prefix("Behaviour"))
            .or_else(|| body.strip_prefix("behavior"))
            .or_else(|| body.strip_prefix("behaviour"))
            .unwrap_or(body);
        let body = body.trim_start_matches(|c: char| c == '=' || c.is_whitespace());
        format!(
            "Name = \"{}\"\nBehaviour = {}\n",
            meta.name.replace('"', "\\\""),
            body
        )
    } else {
        raw
    };

    // Write-free parse: tokenize synthetic source via a temp approach — parse from string.
    let file = parse_npc_source(root, behavior_path, &synthetic)?;
    let mut pending = lower_npc(file)?;
    apply_xml_meta(&mut pending, meta);
    pending.name = meta.name.clone();
    Ok(pending)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn ref_npc_root() -> Option<PathBuf> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../reference/cipsoft-772/runtime/npc");
        p.exists().then_some(p)
    }

    fn goldens_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/npc_import_goldens")
    }

    fn emit_named(root: &Path, stem: &str) -> String {
        let path = root.join(format!("{stem}.npc"));
        let file = parse_npc_file(root, &path).unwrap_or_else(|e| panic!("parse {stem}: {e}"));
        let pending = lower_npc(file).unwrap_or_else(|e| panic!("lower {stem}: {e}"));
        emit_npc_lua(&pending)
    }

    fn assert_golden(name: &str, actual: &str) {
        let path = goldens_dir().join(format!("{name}.lua"));
        if std::env::var("UPDATE_NPC_GOLDENS").as_deref() == Ok("1") {
            std::fs::create_dir_all(goldens_dir()).expect("goldens dir");
            std::fs::write(&path, actual).expect("write golden");
            return;
        }
        let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "missing golden {} ({e}); run with UPDATE_NPC_GOLDENS=1",
                path.display()
            )
        });
        assert_eq!(
            expected, actual,
            "golden mismatch for {name}; UPDATE_NPC_GOLDENS=1 to refresh"
        );
    }

    #[test]
    fn parses_albert() {
        let Some(root) = ref_npc_root() else {
            return;
        };
        let path = root.join("albert.npc");
        let file = parse_npc_file(&root, &path).expect("parse albert");
        assert_eq!(file.name.as_deref(), Some("Albert"));
        assert!(!file.rules.is_empty());
        let pending = lower_npc(file).expect("lower");
        let lua = emit_npc_lua(&pending);
        assert!(lua.contains("NpcType(\"Albert\")"));
        assert!(lua.contains("queued_single_focus"));
    }

    #[test]
    fn golden_albert_quentin_suzy_bank() {
        let Some(root) = ref_npc_root() else {
            eprintln!("skip: reference npc dir missing");
            return;
        };
        assert_golden("albert", &emit_named(&root, "albert"));
        assert_golden("quentin", &emit_named(&root, "quentin"));
        // Suzy includes gen-bank.ndb — cover bank include expansion.
        assert_golden("suzy", &emit_named(&root, "suzy"));
    }

    #[test]
    fn parse_all_reference_corpus() {
        let Some(root) = ref_npc_root() else {
            eprintln!("skip: reference npc dir missing");
            return;
        };
        let pending = import_legacy_root(&root).expect("import all reference npcs");
        assert_eq!(pending.len(), 337, "expected 337 npc files");
        assert!(pending.iter().any(|p| p.name == "Albert"));
        assert!(pending.iter().any(|p| p.name == "Quentin"));
        assert!(pending.iter().any(|p| p.name == "Lokur"));
    }

    #[test]
    fn rejects_string_assignment() {
        let src = r#"
Name = "Bad"
Behaviour = {
"spell" -> String="Find Person", Topic=1
}
"#;
        let root = std::env::temp_dir();
        let err = parse_npc_source(&root, &root.join("bad.npc"), src)
            .and_then(lower_npc)
            .expect_err("string should fail");
        let msg = err.to_string().to_ascii_lowercase();
        assert!(msg.contains("string"), "{msg}");
    }

    #[test]
    fn rejects_bless_town_promote() {
        let root = std::env::temp_dir();
        for (label, src) in [
            (
                "bless",
                r#"
Name = "Bad"
Behaviour = {
"hi" -> Bless(1)
}
"#,
            ),
            (
                "town",
                r#"
Name = "Bad"
Behaviour = {
"hi" -> Town(1)
}
"#,
            ),
            (
                "promote",
                r#"
Name = "Bad"
Behaviour = {
"hi" -> Promote
}
"#,
            ),
        ] {
            let err = parse_npc_source(&root, &root.join("bad.npc"), src)
                .and_then(lower_npc)
                .expect_err(&format!("{label} should fail"));
            let msg = err.to_string().to_ascii_lowercase();
            assert!(
                msg.contains(label),
                "expected {label} in error, got {msg}"
            );
        }
    }

    #[test]
    fn parses_topic_after_assign() {
        let src = r#"
Name = "X"
Behaviour = {
"netlios" -> "questions?", Topic=1

Topic=1,"yes" -> Price=500, "hi", Topic=2
}
"#;
        let root = std::env::temp_dir();
        let file = parse_npc_source(&root, &root.join("x.npc"), src).expect("parse");
        assert_eq!(file.rules.len(), 2, "{:?}", file.rules);
    }
}

