//! Offline one-way importer: legacy `.npc`/`.ndb` → `NpcType`/`NpcDialogue` Lua.
//!
//! Usage (from repo root):
//! ```text
//! cargo run -p tfs-rust-lua --bin import-npcs -- \
//!   --root reference/cipsoft-772/runtime/npc \
//!   --out data/npc/scripts \
//!   --validate-data-dir data \
//!   --keep-extra
//! ```
//!
//! Decompile `--root` imports keep `Type=` literals as written. Those ids are
//! the `items.ron` ids. Included `.ndb` files are emitted once under
//! `--catalog-out` (default: the sibling `catalogs/` of `--out`).
//!
//! Writes to a temp directory under `--out`, validates via [`LuaRuntime`], then
//! atomically replaces generated `*.lua` files (preserves hand-authored files
//! that were not overwritten only when using `--keep-extra`; default replaces
//! the whole definitions set of generated names).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::npc_import::{
    catalog_filename, collect_catalogs, definition_filename, emit_catalog_lua, emit_npc_lua,
    import_legacy_root, import_split_xml,
};
use tfs_rust_content::npcs::PendingNpcDefinition;
use tfs_rust_lua::LuaRuntime;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("import-npcs: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    let opts = parse_args(&args)?;

    let items_dir = opts
        .items_dir
        .clone()
        .or_else(|| opts.validate_data_dir.clone())
        .unwrap_or_else(|| PathBuf::from("data"));

    let items = if opts.root.is_some() || opts.validate_data_dir.is_some() {
        Some(load_items(&items_dir)?)
    } else {
        None
    };

    let pending = if let Some(ref xml) = opts.split_xml {
        let behavior = opts
            .behavior_dir
            .clone()
            .unwrap_or_else(|| xml.join("behavior"));
        // Archive/TVP split files already use OTB server ids — do not remap.
        import_split_xml(xml, &behavior, None).map_err(|e| e.to_string())?
    } else {
        let root = opts
            .root
            .ok_or_else(|| "missing --root <legacy-npc-dir> (or use --split-xml)".to_string())?;
        let items_ref = items.as_ref().ok_or_else(|| {
            "CipSoft --root import requires items (pass --validate-data-dir or --items-dir)"
                .to_string()
        })?;
        import_legacy_root(&root, Some(items_ref)).map_err(|e| e.to_string())?
    };

    let out = opts
        .out
        .ok_or_else(|| "missing --out <definitions-dir>".to_string())?;
    let catalog_out = opts
        .catalog_out
        .clone()
        .unwrap_or_else(|| out.parent().unwrap_or(Path::new(".")).join("catalogs"));

    let catalogs = collect_catalogs(&pending)?;
    let staging = staging_dir(&out)?;
    let staging_scripts = staging.join("scripts");
    let staging_catalogs = staging.join("catalogs");
    fs::create_dir_all(&staging_scripts).map_err(|e| e.to_string())?;
    fs::create_dir_all(&staging_catalogs).map_err(|e| e.to_string())?;
    write_definitions(&staging_scripts, &pending)?;
    write_catalogs(&staging_catalogs, &catalogs)?;

    if let Some(ref data_dir) = opts.validate_data_dir {
        let items_ref = items
            .as_ref()
            .ok_or_else(|| "internal: items required for validate-data-dir".to_string())?;
        validate_staging(
            &staging_scripts,
            &staging_catalogs,
            data_dir,
            items_ref,
            pending.len(),
        )?;
    }

    if opts.dry_run {
        let _ = fs::remove_dir_all(&staging);
        return Ok(format!(
            "dry-run ok: {} NPC(s) and {} catalog(s) parsed/emitted{}; not written to {}",
            pending.len(),
            catalogs.len(),
            if opts.validate_data_dir.is_some() {
                " and Lua-validated"
            } else {
                ""
            },
            out.display()
        ));
    }

    commit_staging(&out, &staging_scripts, &pending, opts.keep_extra)?;
    commit_catalogs(&catalog_out, &staging_catalogs, &catalogs)?;
    let _ = fs::remove_dir_all(&staging);
    Ok(format!(
        "imported {} NPC definition(s) → {} and {} catalog(s) → {}",
        pending.len(),
        out.display(),
        catalogs.len(),
        catalog_out.display()
    ))
}

#[derive(Default)]
struct Opts {
    root: Option<PathBuf>,
    out: Option<PathBuf>,
    catalog_out: Option<PathBuf>,
    validate_data_dir: Option<PathBuf>,
    items_dir: Option<PathBuf>,
    split_xml: Option<PathBuf>,
    behavior_dir: Option<PathBuf>,
    dry_run: bool,
    keep_extra: bool,
}

fn parse_args(args: &[String]) -> Result<Opts, String> {
    let mut opts = Opts::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--root" => {
                i += 1;
                opts.root = Some(require_path(args, i, "--root")?);
            }
            "--out" => {
                i += 1;
                opts.out = Some(require_path(args, i, "--out")?);
            }
            "--catalog-out" => {
                i += 1;
                opts.catalog_out = Some(require_path(args, i, "--catalog-out")?);
            }
            "--validate-data-dir" => {
                i += 1;
                opts.validate_data_dir = Some(require_path(args, i, "--validate-data-dir")?);
            }
            "--items-dir" => {
                i += 1;
                opts.items_dir = Some(require_path(args, i, "--items-dir")?);
            }
            "--split-xml" => {
                i += 1;
                opts.split_xml = Some(require_path(args, i, "--split-xml")?);
            }
            "--behavior-dir" => {
                i += 1;
                opts.behavior_dir = Some(require_path(args, i, "--behavior-dir")?);
            }
            "--dry-run" => opts.dry_run = true,
            "--keep-extra" => opts.keep_extra = true,
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }
    Ok(opts)
}

fn require_path(args: &[String], i: usize, flag: &str) -> Result<PathBuf, String> {
    args.get(i)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} requires a path argument"))
}

fn print_help() {
    eprintln!(
        "\
import-npcs — offline 772 .npc/.ndb → NpcType/NpcDialogue Lua

Options:
  --root <dir>                 Full legacy NPC directory (Type= ids kept as written)
  --split-xml <dir>            Split data/npc XML mode (server ids; no remap)
  --behavior-dir <dir>         Behavior files for --split-xml (default: <xml>/behavior)
  --out <dir>                  Output NPC definitions directory
  --catalog-out <dir>          Output catalog directory (default: <out>/../catalogs)
  --validate-data-dir <data>   Load items from <data>/items and Lua-validate staging
  --items-dir <data>           Items root (default: data or validate-data-dir)
  --dry-run                    Parse/emit/validate only; do not write --out
  --keep-extra                 Keep existing .lua files not produced by this import
  -h, --help                   Show this help
"
    );
}

fn load_items(data_dir: &Path) -> Result<ItemDatabase, String> {
    let ron = ItemDatabase::ron_path(data_dir, tfs_rust_common::ProtocolVersion::V772);
    if !ron.exists() {
        return Err(format!("items.ron not found: {}", ron.display()));
    }
    ItemDatabase::load_ron(&ron).map_err(|e| format!("load items: {e}"))
}

fn staging_dir(out: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(out).map_err(|e| format!("create {}: {e}", out.display()))?;
    let staging = out.join(format!(".import-npcs-staging-{}", std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    Ok(staging)
}

fn write_catalogs(
    dir: &Path,
    catalogs: &std::collections::BTreeMap<String, Vec<tfs_rust_content::npcs::DialogueRule>>,
) -> Result<(), String> {
    for (key, rules) in catalogs {
        let ndb = format!("{key}.ndb");
        let path = dir.join(catalog_filename(&ndb));
        let lua = emit_catalog_lua(&ndb, rules);
        fs::write(&path, lua).map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    Ok(())
}

fn write_definitions(dir: &Path, pending: &[PendingNpcDefinition]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for p in pending {
        let name = definition_filename(&p.name);
        if !seen.insert(name.clone()) {
            return Err(format!("duplicate output filename {name} for {:?}", p.name));
        }
        let path = dir.join(&name);
        let lua = emit_npc_lua(p);
        fs::write(&path, lua).map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    Ok(())
}

fn validate_staging(
    staging_scripts: &Path,
    staging_catalogs: &Path,
    _data_dir: &Path,
    items: &ItemDatabase,
    expected_count: usize,
) -> Result<(), String> {
    let mut rt = LuaRuntime::new().map_err(|e| format!("LuaRuntime: {e}"))?;
    let db = rt
        .load_npc_definitions_dir(staging_scripts, Some(staging_catalogs), items)
        .map_err(|e| format!("Lua validate failed: {e}"))?;
    if db.len() != expected_count {
        return Err(format!(
            "validated {} definitions but imported {expected_count}",
            db.len()
        ));
    }
    Ok(())
}

fn commit_staging(
    out: &Path,
    staging: &Path,
    pending: &[PendingNpcDefinition],
    keep_extra: bool,
) -> Result<(), String> {
    let generated: HashSet<String> = pending
        .iter()
        .map(|p| definition_filename(&p.name))
        .collect();

    if !keep_extra {
        if out.exists() {
            for ent in fs::read_dir(out).map_err(|e| e.to_string())? {
                let ent = ent.map_err(|e| e.to_string())?;
                let path = ent.path();
                if path.extension().and_then(|e| e.to_str()) != Some("lua") {
                    continue;
                }
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.starts_with('.') {
                    continue;
                }
                fs::remove_file(&path).map_err(|e| format!("remove {}: {e}", path.display()))?;
            }
        }
    } else {
        for name in &generated {
            let path = out.join(name);
            if path.exists() {
                fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }
    }

    for ent in fs::read_dir(staging).map_err(|e| e.to_string())? {
        let ent = ent.map_err(|e| e.to_string())?;
        let from = ent.path();
        let name = from
            .file_name()
            .ok_or_else(|| "staging entry without name".to_string())?;
        let to = out.join(name);
        fs::rename(&from, &to)
            .or_else(|_| {
                fs::copy(&from, &to)
                    .map(|_| ())
                    .and_then(|_| fs::remove_file(&from))
            })
            .map_err(|e| format!("commit {}: {e}", to.display()))?;
    }
    Ok(())
}

fn commit_catalogs(
    out: &Path,
    staging: &Path,
    catalogs: &std::collections::BTreeMap<String, Vec<tfs_rust_content::npcs::DialogueRule>>,
) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|e| format!("create {}: {e}", out.display()))?;
    let generated: HashSet<String> = catalogs
        .keys()
        .map(|key| catalog_filename(&format!("{key}.ndb")))
        .collect();
    if out.exists() {
        for ent in fs::read_dir(out).map_err(|e| e.to_string())? {
            let ent = ent.map_err(|e| e.to_string())?;
            let path = ent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("lua") {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !generated.contains(name) {
                fs::remove_file(&path).map_err(|e| format!("remove {}: {e}", path.display()))?;
            }
        }
    }
    for name in &generated {
        let from = staging.join(name);
        let to = out.join(name);
        if to.exists() {
            fs::remove_file(&to).map_err(|e| e.to_string())?;
        }
        fs::rename(&from, &to)
            .or_else(|_| {
                fs::copy(&from, &to)
                    .map(|_| ())
                    .and_then(|_| fs::remove_file(&from))
            })
            .map_err(|e| format!("commit {}: {e}", to.display()))?;
    }
    Ok(())
}
