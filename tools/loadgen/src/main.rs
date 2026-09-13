//! Open-loop 772 loadgen CLI (`tfs-loadgen`).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use tfs_loadgen::encode::wrap_tcp_frame;
use tfs_loadgen::inbound::InboundState;
use tfs_loadgen::item_extra::shared_item_extra;
use tfs_loadgen::latency::{LatencySet, RunReport};
use tfs_loadgen::ramp::LoginGate;
use tfs_loadgen::roles::bot_seed;
use tfs_loadgen::scenario::{BotRng, Scenario};
use tfs_loadgen::session::{BotConfig, load_rsa_pem, run_bot, v772_caps};
use tokio::net::TcpStream;
use tokio::task::JoinSet;

#[derive(Parser, Debug)]
#[command(name = "tfs-loadgen", about = "Open-loop 772 client loadgen")]
struct Args {
    /// Concurrent bots (overrides scenario `bots` when set).
    #[arg(long)]
    bots: Option<usize>,
    /// RON scenario under `bench/scenarios/`.
    #[arg(long)]
    scenario: Option<PathBuf>,
    /// Spike: one bot alternating walk north/south.
    #[arg(long)]
    walk_ns: bool,
    /// Measurement window in seconds (overrides scenario `duration_s` when set).
    #[arg(long)]
    duration_s: Option<u64>,
    /// Spike walk count (implies `--walk-ns`).
    #[arg(long)]
    beats: Option<u32>,
    #[arg(long, default_value = "127.0.0.1:7171")]
    login: String,
    #[arg(long, default_value = "127.0.0.1:7172")]
    game: String,
    #[arg(long, default_value_t = 1)]
    account: u32,
    #[arg(long, default_value = "1")]
    password: String,
    #[arg(long, default_value = "Test")]
    character: String,
    #[arg(long)]
    rsa: Option<PathBuf>,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long, default_value_t = 42)]
    seed: u64,
    #[arg(long, default_value_t = 200)]
    walk_period_ms: u64,
    /// Blast length-framed packets at a null echo (no RSA / login).
    #[arg(long)]
    echo_ceiling: bool,
    #[arg(long, default_value = "127.0.0.1:17171")]
    echo_addr: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if args.echo_ceiling {
        return run_echo_ceiling(&args).await;
    }

    let duration_s = args.duration_s.unwrap_or(30);
    let mut scenario = if let Some(path) = &args.scenario {
        Scenario::load_path(path)?
    } else {
        Scenario::walker_spike(duration_s, args.walk_period_ms, args.seed)
    };
    if args.walk_ns || args.beats.is_some() {
        scenario = Scenario::walker_spike(duration_s, args.walk_period_ms, args.seed);
    }
    if let Some(d) = args.duration_s {
        scenario.duration_s = d;
    }
    if let Some(beats) = args.beats {
        let need_s = (u64::from(beats) * args.walk_period_ms).div_ceil(1000) + 2;
        scenario.duration_s = scenario.duration_s.max(need_s);
    }
    let bots = args
        .bots
        .unwrap_or(if args.scenario.is_some() {
            scenario.bots
        } else {
            1
        })
        .max(1);
    scenario.bots = bots;

    let key = Arc::new(load_rsa_pem(args.rsa.as_deref())?);
    let gate = Arc::new(LoginGate::phase_d_default());
    let caps = v772_caps();
    let bounce_ns = args.walk_ns || args.beats.is_some();
    let item_extra = shared_item_extra();

    let mut set = JoinSet::new();
    for i in 0..bots {
        let mut rng = BotRng::new(bot_seed(scenario.seed, i));
        let role = scenario.pick_role(i, &mut rng);
        let character = if i == 0 {
            args.character.clone()
        } else {
            format!("{}{i}", args.character)
        };
        let cfg = BotConfig {
            index: i,
            login_addr: args.login.clone(),
            game_addr: args.game.clone(),
            account: args.account.saturating_add(i as u32),
            password: args.password.clone(),
            character,
            scenario: scenario.clone(),
            role,
            bounce_ns,
            walk_count: args.beats,
            item_extra: Arc::clone(&item_extra),
        };
        let key = Arc::clone(&key);
        let gate = Arc::clone(&gate);
        set.spawn(async move { run_bot(cfg, key, gate, caps).await });
    }

    let mut merged = LatencySet::new()?;
    let mut inbound = InboundState::default();
    let mut bytes_out = 0u64;
    let mut sends = 0u64;
    let mut outstanding = 0u64;
    let mut ok = 0usize;
    while let Some(joined) = set.join_next().await {
        let outcome = joined.map_err(|e| anyhow!("bot join: {e}"))??;
        merged.add(&outcome.latency)?;
        inbound.add_counters(&outcome.inbound);
        bytes_out += outcome.bytes_out;
        sends += outcome.sends;
        outstanding += outcome.latency.outstanding_count();
        ok += 1;
    }

    let report = RunReport {
        bots: ok,
        duration_s: scenario.duration_s,
        warmup_s: scenario.warmup_s,
        walk: merged.walk_summary(),
        spell_rune: merged.spell_summary(),
        bytes_in: inbound.bytes_in,
        bytes_out,
        outstanding_at_end: outstanding,
        sends,
        magic_effects: inbound.magic_effects,
        animated_texts: inbound.animated_texts,
        damage_sum: inbound.damage_sum,
        damage_samples: inbound.damage_samples,
        distance_shoots: inbound.distance_shoots,
        creature_health: inbound.creature_health,
        other_creature_moves: inbound.other_creature_moves,
        unique_creatures: inbound.unique_creatures(),
        bytes_discarded: inbound.bytes_discarded,
        skip_failures: inbound.skip_failures,
    };
    let json = report.to_json();
    print!("{json}");
    if let Some(path) = args.out {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, &json).with_context(|| format!("write {}", path.display()))?;
    }
    Ok(())
}

async fn run_echo_ceiling(args: &Args) -> Result<()> {
    let payload = vec![0u8; 16];
    let frame = wrap_tcp_frame(&payload);
    let bots = args.bots.unwrap_or(1).max(1);
    let duration_s = args.duration_s.unwrap_or(30);
    let duration = std::time::Duration::from_secs(duration_s.max(1));
    let mut set = JoinSet::new();
    let start = Instant::now();
    for _ in 0..bots {
        let addr = args.echo_addr.clone();
        let frame = frame.clone();
        set.spawn(async move {
            let mut sock = TcpStream::connect(&addr)
                .await
                .with_context(|| format!("connect echo {addr}"))?;
            use tokio::io::AsyncWriteExt;
            let mut sends = 0u64;
            let mut bytes = 0u64;
            let deadline = Instant::now() + duration;
            while Instant::now() < deadline {
                sock.write_all(&frame).await?;
                sends += 1;
                bytes += frame.len() as u64;
            }
            Ok::<_, anyhow::Error>((sends, bytes))
        });
    }
    let mut sends = 0u64;
    let mut bytes = 0u64;
    while let Some(j) = set.join_next().await {
        let (s, b) = j.map_err(|e| anyhow!("echo bot join: {e}"))??;
        sends += s;
        bytes += b;
    }
    let elapsed = start.elapsed().as_secs_f64().max(0.001);
    println!(
        "{{\n  \"mode\": \"echo_ceiling\",\n  \"bots\": {bots},\n  \"duration_s\": {},\n  \"sends\": {sends},\n  \"bytes_out\": {bytes},\n  \"sends_per_s\": {:.1}\n}}",
        args.duration_s.unwrap_or(30),
        sends as f64 / elapsed
    );
    Ok(())
}
