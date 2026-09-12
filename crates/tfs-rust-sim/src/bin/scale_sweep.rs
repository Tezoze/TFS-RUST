//! Tier 2 scaling sweep binary — saturates the game thread via `advance_beat`.
//!
//! Not a parity contract. Usage:
//! `scale_sweep --axis monsters|players|spectators [--points 50,100,200,400,800]
//!              [--beats 600] [--warmup 100] [--map synthetic|otbm] [--seed 42]
//!              [--out results/sweep_<axis>.json]`

use std::env;
use std::fs;
use std::path::PathBuf;

use tfs_rust_sim::sweep::{SweepAxis, SweepMap, run_sweep};
use tfs_rust_sim::world::test_runtime;

fn main() {
    if let Err(e) = run_main() {
        eprintln!("scale_sweep: {e}");
        std::process::exit(1);
    }
}

fn run_main() -> Result<(), String> {
    let _guard = test_runtime().enter();
    let args: Vec<String> = env::args().skip(1).collect();
    const USAGE: &str = "usage: scale_sweep --axis monsters|players|spectators [--points 50,100,200,400,800] [--beats 600] [--warmup 100] [--map synthetic|otbm] [--seed 42] [--out PATH]";
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return Ok(());
    }

    let mut axis = None;
    let mut points = vec![50usize, 100, 200, 400, 800];
    let mut beats = 600u32;
    let mut warmup = 100u32;
    let mut map = SweepMap::Synthetic;
    let mut seed = 42u32;
    let mut out: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--axis" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--axis requires monsters|players|spectators".to_string())?;
                axis = Some(SweepAxis::parse(v)?);
                i += 2;
            }
            "--points" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--points requires a comma-separated list".to_string())?;
                points = parse_points(v)?;
                i += 2;
            }
            "--beats" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--beats requires an integer".to_string())?;
                beats = v.parse().map_err(|_| format!("invalid --beats {v}"))?;
                i += 2;
            }
            "--warmup" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--warmup requires an integer".to_string())?;
                warmup = v.parse().map_err(|_| format!("invalid --warmup {v}"))?;
                i += 2;
            }
            "--map" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--map requires synthetic|otbm".to_string())?;
                map = SweepMap::parse(v)?;
                i += 2;
            }
            "--seed" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--seed requires an integer".to_string())?;
                seed = v.parse().map_err(|_| format!("invalid --seed {v}"))?;
                i += 2;
            }
            "--out" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--out requires a path".to_string())?;
                out = Some(PathBuf::from(v));
                i += 2;
            }
            other => return Err(format!("unexpected argument: {other}\n{USAGE}")),
        }
    }

    let axis = axis.ok_or_else(|| format!("--axis is required\n{USAGE}"))?;
    let out = out.unwrap_or_else(|| PathBuf::from(format!("results/sweep_{}.json", axis.as_str())));

    eprintln!(
        "scale_sweep: axis={} points={points:?} beats={beats} warmup={warmup} map={} seed={seed}",
        axis.as_str(),
        map.as_str()
    );

    let report = run_sweep(axis, &points, beats, warmup, map, seed)?;
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    fs::write(&out, report.to_json()).map_err(|e| format!("write {}: {e}", out.display()))?;

    for p in &report.points {
        eprintln!(
            "  n={:<4} beat_wall_us p50={} p99={} max={} path_searches={} out_bytes/beat={}",
            p.n,
            p.beat_wall_us_p50,
            p.beat_wall_us_p99,
            p.beat_wall_us_max,
            p.path_searches,
            p.outgoing_bytes_per_beat
        );
    }
    eprintln!("wrote {}", out.display());
    Ok(())
}

fn parse_points(s: &str) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let n: usize = part
            .parse()
            .map_err(|_| format!("invalid --points entry {part}"))?;
        if n == 0 {
            return Err("--points entries must be > 0".into());
        }
        out.push(n);
    }
    if out.is_empty() {
        return Err("--points must list at least one N".into());
    }
    Ok(out)
}
