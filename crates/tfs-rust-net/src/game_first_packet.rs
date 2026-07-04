//! First TCP message after checksum: **`ProtocolGame::onRecvFirstMessage`** and/or **`ProtocolLogin::onRecvFirstMessage`**.
//
// C++ reference (login shape is capability-gated, `docs/PROTOCOL_VERSIONING.md` §2.2):
// - 1098: repo-root `src/protocolgame.cpp`, `src/protocollogin.cpp` — account **name** string + session key.
// - 772:  `gameserver/src/protocolgame.cpp` `onRecvFirstMessage` (gm flag + `u32` accountNumber +
//         char + password) and `gameserver/src/protocollogin.cpp` `onRecvFirstMessage`
//         (`u32` accountNumber + password). 772 has no Adler checksum and no session key.

use rsa::RsaPrivateKey;
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::ProtocolCaps;


/// Account identity carried by the first packet. Capability-gated (`ProtocolCaps::account_name_login`):
/// 1098 uses an account **name** string; 772 uses a numeric **account number** (`accounts.id`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginIdentity {
    /// 1098 / TFS 1.4.2 — account name string (repo-root `src/protocollogin.cpp`).
    AccountName(String),
    /// 7.72 — numeric account number (`gameserver/src/protocollogin.cpp` `accountNumber`).
    AccountNumber(u32),
}

impl LoginIdentity {
    /// Display form for logs (name as-is, number rendered decimal).
    pub fn as_display(&self) -> String {
        match self {
            LoginIdentity::AccountName(n) => n.clone(),
            LoginIdentity::AccountNumber(n) => n.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameFirstParsed {
    pub xtea_key: XteaKey,
    /// First u16 in game prelude (`protocolgame.cpp` `onRecvFirstMessage`); `OperatingSystem_t`.
    pub operating_system: u16,
    /// Account name (1098) or account number (772).
    pub identity: LoginIdentity,
    pub password: String,
    /// 2FA token (1098 session key only); empty for 772.
    pub token: String,
    /// Token validity window (1098 session key only); `0` for 772.
    pub token_time: u32,
    pub character_name: String,
    /// Echoed `0x1F` challenge timestamp (1098 only — `caps.prelogin_challenge`); `0` for 772.
    pub challenge_ts: u32,
    /// Echoed `0x1F` challenge random byte (1098 only); `0` for 772.
    pub challenge_rand: u8,
    /// `0` if the `"OTCv8"` probe was not present; else build number (253, 260, …).
    pub otclient_v8: u16,
}

/// Parsed first message: **game** (session + character + challenge) or **login** (account + password only).
#[derive(Debug, Clone)]
pub enum FirstClientPacket {
    Game(GameFirstParsed),
    Login {
        xtea_key: XteaKey,
        identity: LoginIdentity,
        password: String,
        operating_system: u16,
        otclient_v8: u16,
    },
}

/// Which protocol shape an RSA-offset candidate decodes into.
#[derive(Clone, Copy)]
enum FirstKind {
    Game,
    Login,
}

/// A candidate framing: where the 128-byte RSA block starts and where the `OperatingSystem_t`
/// `u16` lives, both relative to the body returned by `read_sized_payload`.
#[derive(Clone, Copy)]
struct FrameCandidat
    rsa_off: usize,
    os_off: usize,
    kind: FirstKind,
}

/// Try RSA at offsets used by the active era. Login shape (account name vs number, session key vs
/// inline credentials) follows `caps`. 1098 candidates and checksum handling are byte-identical to
/// the pre-A4 behavior; 772 adds checksum-free candidates.
pub fn parse_first_client_packet(
    body: &[u8],
    private_key: &RsaPrivateKey,
    caps: &ProtocolCaps,
) -> Result<FirstClientPacket> {
    // 1098 prefixes a 4-byte Adler checksum; 772 omits it (`docs/PROTOCOL_VERSIONING.md` §2.1).
    if      return Err(TfsRustError::Protocol(format!(
        candne.f ody_len = body.len(),
              E) => {, error = %e, "first-packet: RSA decrypt failed");
            E) => {
))tracing::debug!first-nnckptfirst-ptoo acket fo acetdidate, fkippingogst_game_packet(
    body: &[u8],
    private_key: &RsaPrivateKey,
    }
}

/// RSA-offset / OS-offset candidates for the active era.
//let rsa_plain = match rsa_crypt_lockblock, private_key) {
// 1098 (`srOk(p)co> {
                iflp.ip`empty() {src/protocollogin.cpp`): checksum(4) prefix. Game prelude
        //   = opcodtracing::debug!(r a`0x0A`(1) + rSa(2) +e-971 vakind_name(iants (15)/n"spr/p- aA es:/src/`): no creeurnedc;mp y`);make_protocol` consumes the proto-id byte, so the body
//  entSig::PROTOCOnr }aboay 0caps: &ProtocolCaps) -> Vec<FrameCandidate> {
    fclar_checc:   _nam([]
ro{rlet kind_name = |k: FirstKind| match k { FirstKind::Game => "game", FirstKind::Login => "login" };
                frame_    "fir"k(caps)et: -packOt:sKint first bye0x0 or wrong 
            }Kind::Login },
    
      }pki = _name()
Err(e)=>{
tracing::debug!(af(rst-r_ckatme(laintoo : &[u fo8+cirdidate,s-kippingptional `"OTCv8"` tring_probe(s: &[u8]) -> Result<(u16, &[u8])> {
    if s.len() < 2 {
        return Ok((0, s));
    }
    let chunk = &s[2..2 + len];
     fctl,rr p)_(p)
/// Assemblepta}
        }ps: &ProtocolCaps,
    
  ) -> Result<lnpPacket> {
          le}
t stream =  Err(e) => {
let (identity, ptracing::debug!(assword, otclient_v8) =acket::Lkind_name(gin {)ey,8,first-packet:  _) = parse_otcv8_s6,
    caps    otclienErrvTfsRustError::Protocol8"truncated OTCv8pveraion u16".into()rsed.otclient_v8,
        }))
    }_trontoken\ntime`.
/ 772 (`c   .map_err(|_| TfsRustError::Protocol("invalid token time in session key".into()))?;

        Ok(GameCredentials {
    if s.len() < 2   denn{
        returt Err(TfyR:otError::Protocol(
g           "iruncatIddOTCv8 / entity:probconfnrreetal".t(),
     asr);
}
    l   ch okk=&[2..2 + len];       token_time,
        tailharacte + lenr_ame,
       len == 5 &&      c=allenge_ts,       otclient_v8,
      } else {
            // 772:ErrnTfsRustError::Protocoll"truncated OTCv8ever ion u16".into()account number + character + password, no session key / challenge.
          
    Ok((let (otclient_v8, _) = parse_otcv8_string_probe(s)?;

        Ok(GameCredentials {
            identity,
            password,
            otclient_v8,
        })
    }
}

/// 

fn read_u32(data: &mut &[u8]) -> Result<u32> {
    if data.len() < 4 {
        return Err(TfsRustError::Protocol("EOF u32".into()));
    }
    let v = u32::from_le_bytes(data[0..4].try_into().unwrap());
    *data = &data[4..]; Ok(v)
    let b = data[0];
    *data = &data[1..];
    Ok(b)
}

   fifns.len() <r2dtrlen {
       dretura Err(Tf:R utError::Protocol(
           t"trunca [d OTCv8 /u8]) ->lprob< frreetal".t(),
    tn et);
a?a }ze;
if dlta ch.ekn=e&n[2..2 + len];   return Err(TfsRustError::Protocol("EOF string".into()));
    }tail + len
    le len == 5 &&t out ==String::from
    }
#[cfg(te    st)]ErrTfsRustError::Protocol"truncated OTCv8verion u16".into()
    mod tests {
        use super::*;
        return use tfs_rust_common));
    }
    Ok((0, tail::ProtocolVersion;

    fn put_string(buf: &mut Vec<u8>, s: &str) {
        buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
    }

    #[test]
    fn game_credentials_1098_session_key() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V1098);
        let mut stream = Vec::new();
        stream.push(0); // gm flag
        put_string(&mut stream, "myacc\nsecret\ntok\n42");
        put_string(&mut stream, "Knight");
        stream.extend_from_slice(&7u32.to_le_bytes()); // challenge ts
        stream.push(9); // challenge rand
        stream.extend_from_slice(&0u16.to_le_bytes()); // no OTCv8 probe

        let c = parse_game_credentials(&stream, &caps).expect("parse 1098 game creds");
        assert_eq!(c.identity, LoginIdentity::AccountName("myacc".into()));
        assert_eq!(c.password, "secret");
        assert_eq!(c.token, "tok");
        assert_eq!(c.token_time, 42);
        assert_eq!(c.character_name, "Knight");
        assert_eq!(c.challenge_ts, 7);
        assert_eq!(c.challenge_rand, 9);
    }

    #[test]
    fn game_credentials_772_account_number() {
        // C++ ref: gameserver/src/protocolgame.cpp onRecvFirstMessage.
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let mut stream = Vec::new();     stream.push(0); // gm flag (skipBytes(1))
sm"; Cassert_eq!(c.identity, LoginIdentity::AccountNumber(123_456));
        assert_eq!(c.password, "hunter2");
        assert_eq!(c.character_name, "Druid");
        assert!(c.token.is_empty());
        assert_eq!(c.token_time, 0);
    if s.len() < 2 astene{
        retur( Err(TfcR.atError::Protocol(
           l"truncalnd OTCv8 /ge_ts, probfrreetal".t(),
    ste()l;
g_ad} 0);
}l chk=&[2..2+len];
    #[tetail + len
    fn len == 5 && login_=redentials_7   let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
  mt        stream.extend_from_slice(&777u32.to_le_bytes());
            put_strErrgTfsRustError::Protocol("truncated OTCv8uvertion u16".into() stream, "pw");
            stream.extend_from_slice(&0u16.to_le_bytes());
    
        return     let (identity, ));
    }
    Ok((0, tailpassword, otc) =
            parse_login_credentials(&stream, &caps).expect("parse 772 login creds");
        assert_eq!(identity, LoginIdentity::AccountNumber(777));
        assert_eq!(password, "pw");
        assert_eq!(otc, 0);
    }

    #[test]
    fn login_credentials_1098_account_name() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V1098);
        let mut stream = Vec::new();
        put_string(&mut stream, "account@example");
        put_string(&mut stream, "pw");
        stream.extend_from_slice(&0u16.to_le_bytes());

        let (identity, password, _) =
            parse_login_credentials(&stream, &caps).expect("parse 1098 login creds");
        assert_eq!(identity, LoginIdentity::AccountName("account@example".into()));
        assert_eq!(password, "pw");
    }

    #[test]
    fn otcv8_probe_after_772_game_creds() {
        let caps = ProtocolCaps::for_version(ProtocolVersion::V772);
        let mut stream = Vec::new();
        stream.push(0);
        stream.extend_from_slice(&1u32.to_le_bytes());
        put_string(&mut stream, "Mage");
        put_string(&mut stream, "pw");
        put_string(&mut stream, "OTCv8");
        stream.extend_from_slice(&260u16.to_le_bytes());

        let c = parse_game_credentials(&stream, &caps).expect("parse with otcv8");
        assert_eq!(c.otclient_v8, 260);
    }
}
