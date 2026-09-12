//! Login 7171 → char list → game 7172 → XTEA open-loop.
//!
//! C++ reference: `gameserver/src/protocollogin.cpp` `getCharacterList`;
//! `protocolgame.cpp` `onRecvFirstMessage` (no 772 challenge).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use rsa::RsaPrivateKey;
use tfs_rust_common::{Position, ProtocolCaps, ProtocolVersion};
use tfs_rust_net::game_frame::read_sized_payload;
use tfs_rust_net::protocol_game::{decrypt_xtea_game_body, encrypt_xtea_game_frame};
use tfs_rust_net::rsa::{private_key_from_pkcs1_pem, public_parts};
use tfs_rust_net::xtea_tfs::{self, RoundKeys};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::encode::{
    PROTOCOL_772, STOCK_CLIENT_OS, encode_attack, encode_game_first, encode_login_first,
    encode_look_at, encode_ping_reply, encode_say, encode_use_item_ex, encode_walk, wrap_tcp_frame,
};
use crate::inbound::{InboundEvent, InboundState};
use crate::latency::LatencySet;
use crate::ramp::LoginGate;
use crate::roles::{bot_seed, fill_schedule};
use crate::scenario::{BotRng, RoleKind, Scenario};
use crate::scheduler::{ActionKind, OpenLoop};

const CHAR_LIST: u8 = 0x64;
const LOGIN_MOTD: u8 = 0x14;
const LOGIN_ERR: u8 = 0x0A;

pub struct BotConfig {
    pub index: usize,
    pub login_addr: String,
    pub game_addr: String,
    pub account: u32,
    pub password: String,
    pub character: String,
    pub scenario: Scenario,
    pub role: RoleKind,
    pub bounce_ns: bool,
    pub walk_count: Option<u32>,
}

pub struct BotOutcome {
    pub latency: LatencySet,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub sends: u64,
    pub inbound: InboundState,
}

pub fn load_rsa_pem(explicit: Option<&Path>) -> Result<RsaPrivateKey> {
    let path = resolve_pem_path(explicit)?;
    let pem = std::fs::read_to_string(&path)
        .with_context(|| format!("read RSA PEM {}", path.display()))?;
    private_key_from_pkcs1_pem(&pem).map_err(|e| anyhow!("parse RSA PEM: {e}"))
}

fn resolve_pem_path(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }
    if let Ok(p) = std::env::var("TFS_RSA_PEM") {
        return Ok(PathBuf::from(p));
    }
    let candidates = [
        PathBuf::from("key.pem"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../key.pem"),
    ];
    for p in candidates {
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(anyhow!("RSA PEM not found (pass --rsa or set TFS_RSA_PEM)"))
}

pub async fn run_bot(
    cfg: BotConfig,
    key: Arc<RsaPrivateKey>,
    gate: Arc<LoginGate>,
    caps: ProtocolCaps,
) -> Result<BotOutcome> {
    let (n, e) = public_parts(key.as_ref());
    let mut rng = BotRng::new(bot_seed(cfg.scenario.seed, cfg.index));
    let xtea = [
        rng.next_u64() as u32,
        rng.next_u64() as u32,
        rng.next_u64() as u32,
        rng.next_u64() as u32,
    ];
    let round = xtea_tfs::expand_key(&xtea);

    let login_body = encode_login_first(
        STOCK_CLIENT_OS,
        PROTOCOL_772,
        &xtea,
        cfg.account,
        &cfg.password,
        &n,
        &e,
    )?;

    {
        let _slot = gate.acquire().await?;
        let mut login = TcpStream::connect(&cfg.login_addr)
            .await
            .with_context(|| format!("connect login {}", cfg.login_addr))?;
        let frame = wrap_tcp_frame(&login_body);
        login.write_all(&frame).await?;
        let Some(mut reply) = read_sized_payload(&mut login).await? else {
            return Err(anyhow!("login server closed before char list"));
        };
        let plain = decrypt_xtea_game_body(&mut reply, &round, &caps)
            .map_err(|err| anyhow!("login XTEA: {err}"))?;
        let chars = parse_char_list(plain)?;
        if chars.is_empty() {
            return Err(anyhow!("empty character list"));
        }
        drop(login);
    }

    let game_body = encode_game_first(
        STOCK_CLIENT_OS,
        PROTOCOL_772,
        &xtea,
        0,
        cfg.account,
        &cfg.character,
        &cfg.password,
        &n,
        &e,
    )?;
    let mut game = TcpStream::connect(&cfg.game_addr)
        .await
        .with_context(|| format!("connect game {}", cfg.game_addr))?;
    let gframe = wrap_tcp_frame(&game_body);
    game.write_all(&gframe).await?;
    let mut bytes_out = gframe.len() as u64;

    let (mut reader, mut writer) = game.into_split();
    let mut inbound = InboundState::default();
    let mut latency = LatencySet::new()?;
    let mut sends = 0u64;

    let start = Instant::now();
    let warmup = Duration::from_secs(cfg.scenario.warmup_s);
    let record_from = start + warmup;
    let mut ol = OpenLoop::new();
    if let Some(nwalk) = cfg.walk_count {
        ol.schedule_walk_ns(
            start,
            nwalk,
            Duration::from_millis(cfg.scenario.walk_period_ms.max(1)),
        );
    } else {
        let mut sched = cfg.scenario.clone();
        sched.duration_s = cfg
            .scenario
            .warmup_s
            .saturating_add(cfg.scenario.duration_s);
        fill_schedule(&mut ol, cfg.role, &sched, start, &mut rng, cfg.bounce_ns);
    }
    let run_end = start + warmup + Duration::from_secs(cfg.scenario.duration_s.max(1));

    loop {
        let now = Instant::now();
        if now >= run_end && ol.is_empty() {
            break;
        }
        let until_action = ol.next_intended().unwrap_or(run_end).min(run_end);
        let sleep = until_action.saturating_duration_since(now);

        tokio::select! {
            body = read_sized_payload(&mut reader) => {
                let Some(mut body) = body.map_err(|e| anyhow!("game read: {e}"))? else {
                    break;
                };
                match decrypt_xtea_game_body(&mut body, &round, &caps) {
                    Ok(plain) => {
                        let evs = inbound.feed(plain);
                        let t = Instant::now();
                        for ev in evs {
                            match ev {
                                InboundEvent::WalkAck => latency.on_walk_ack(t),
                                InboundEvent::MagicEffect { pos } => {
                                    latency.on_magic_effect(t, pos);
                                }
                                InboundEvent::Ping(op) => {
                                    let pkt = encode_ping_reply(op);
                                    bytes_out += write_game(&mut writer, &pkt, &round, &caps).await?;
                                }
                                InboundEvent::OtherCreature { .. } => {}
                            }
                        }
                    }
                    Err(_) => {
                        inbound.bytes_in += body.len() as u64;
                        inbound.bytes_discarded += body.len() as u64;
                    }
                }
            }
            _ = tokio::time::sleep(sleep) => {
                let now = Instant::now();
                if now >= run_end {
                    break;
                }
                while let Some(act) = ol.pop_due(now) {
                    let payload = materialize(&act.kind, &inbound);
                    let corr = match &act.kind {
                        ActionKind::Walk(_) => act.kind.correlate(),
                        ActionKind::UseItemEx { .. } | ActionKind::Say(_) => {
                            Some((crate::latency::Correlate::SpellRune, inbound.pos))
                        }
                        _ => None,
                    };
                    bytes_out += write_game(&mut writer, &payload, &round, &caps).await?;
                    sends += 1;
                    if act.intended >= record_from
                        && let Some((kind, tile)) = corr
                    {
                        latency.on_send(kind, act.intended, tile);
                    }
                }
            }
        }
    }

    Ok(BotOutcome {
        latency,
        bytes_in: inbound.bytes_in,
        bytes_out,
        sends,
        inbound,
    })
}

fn materialize(kind: &ActionKind, inbound: &InboundState) -> Vec<u8> {
    match kind {
        ActionKind::Walk(op) => encode_walk(*op),
        ActionKind::Attack(id) => {
            let cid = inbound.last_other_creature_id.unwrap_or(*id);
            encode_attack(cid)
        }
        ActionKind::Say(text) => encode_say(text),
        ActionKind::UseItemEx {
            from,
            from_sprite,
            to,
            to_sprite,
        } => {
            let dest = if to.x == 0 && to.y == 0 {
                inbound.pos.unwrap_or(*to)
            } else if to.x == 2 && to.y == 2 {
                inbound
                    .pos
                    .map(|p| Position::new(p.x.saturating_add(2), p.y.saturating_add(2), p.z))
                    .unwrap_or(*to)
            } else {
                *to
            };
            encode_use_item_ex(*from, *from_sprite, 0, dest, *to_sprite, 0)
        }
        ActionKind::LookAt(_) => {
            let pos = inbound.pos.unwrap_or(Position::new(0, 0, 7));
            encode_look_at(pos, 0, 0)
        }
    }
}

async fn write_game<W: AsyncWriteExt + Unpin>(
    w: &mut W,
    payload: &[u8],
    keys: &RoundKeys,
    caps: &ProtocolCaps,
) -> Result<u64> {
    let frame = encrypt_xtea_game_frame(payload, keys, caps);
    let n = frame.len() as u64;
    w.write_all(&frame).await?;
    Ok(n)
}

/// Classic 772 char list after XTEA decrypt (`protocol_login_out.rs` `build_login_success_classic`).
pub fn parse_char_list(plain: &[u8]) -> Result<Vec<String>> {
    let mut i = 0usize;
    let mut names = Vec::new();
    while i < plain.len() {
        let op = plain[i];
        i += 1;
        match op {
            LOGIN_MOTD => {
                let s = read_tstring(plain, &mut i)?;
                let _ = s;
            }
            LOGIN_ERR => {
                let msg = read_tstring(plain, &mut i)?;
                return Err(anyhow!("login error: {msg}"));
            }
            CHAR_LIST => {
                if i >= plain.len() {
                    return Err(anyhow!("truncated char list count"));
                }
                let n = plain[i] as usize;
                i += 1;
                for _ in 0..n {
                    let name = read_tstring(plain, &mut i)?;
                    let _world = read_tstring(plain, &mut i)?;
                    if plain.len().saturating_sub(i) < 6 {
                        return Err(anyhow!("truncated char list entry"));
                    }
                    i += 4; // ip
                    i += 2; // port
                    names.push(name);
                }
                // premium u16 — ignore
                return Ok(names);
            }
            _ => {
                return Err(anyhow!("unexpected login opcode 0x{op:02x}"));
            }
        }
    }
    Err(anyhow!("no 0x64 char list in login reply"))
}

fn read_tstring(buf: &[u8], i: &mut usize) -> Result<String> {
    if buf.len().saturating_sub(*i) < 2 {
        return Err(anyhow!("truncated string length"));
    }
    let len = u16::from_le_bytes([buf[*i], buf[*i + 1]]) as usize;
    *i += 2;
    if buf.len().saturating_sub(*i) < len {
        return Err(anyhow!("truncated string"));
    }
    let s = String::from_utf8_lossy(&buf[*i..*i + len]).into_owned();
    *i += len;
    Ok(s)
}

pub fn v772_caps() -> ProtocolCaps {
    ProtocolCaps::for_version(ProtocolVersion::V772)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_list_classic() {
        let mut p = vec![CHAR_LIST, 1];
        fn put(buf: &mut Vec<u8>, s: &str) {
            buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
            buf.extend_from_slice(s.as_bytes());
        }
        put(&mut p, "Test");
        put(&mut p, "World");
        p.extend_from_slice(&0u32.to_le_bytes());
        p.extend_from_slice(&7172u16.to_le_bytes());
        p.extend_from_slice(&0u16.to_le_bytes());
        let names = parse_char_list(&p).expect("list");
        assert_eq!(names, vec!["Test".to_string()]);
    }

    #[test]
    fn login_error() {
        let mut p = vec![LOGIN_ERR];
        p.extend_from_slice(&5u16.to_le_bytes());
        p.extend_from_slice(b"nope!");
        let err = parse_char_list(&p).unwrap_err();
        assert!(err.to_string().contains("nope"));
    }
}
