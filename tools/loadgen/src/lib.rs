//! Synthetic 772 client loadgen — open-loop, intended-time latency.
//!
//! C++ reference: `gameserver/src/protocollogin.cpp` / `protocolgame.cpp` first-packet
//! layouts; wire reuse via `tfs-rust-net` (RSA, XTEA, framing). Does not depend on
//! `tfs-rust-core`.

pub mod encode;
pub mod inbound;
pub mod latency;
pub mod ramp;
pub mod roles;
pub mod scenario;
pub mod scheduler;
pub mod session;

pub use encode::{
    PROTOCOL_772, STOCK_CLIENT_OS, encode_game_first, encode_login_first, wrap_tcp_frame,
};
pub use latency::RunReport;
pub use scenario::Scenario;
pub use session::{BotConfig, BotOutcome, load_rsa_pem, run_bot};
