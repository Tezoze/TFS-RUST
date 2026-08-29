# Player-to-player trade (audit 1.1)

**Status:** complete.
**Source:** `docs/772_PARITY_GAP_AUDIT.md` §1.1.

## Delivered

- `crates/tfs-rust-core/src/trade.rs` — `TradeRegistry`, ToDo `TDTrade`, look/accept/reject, `NotifyTrades`, walk cancel, house transfer on accept.
- Wire: `ProtocolCodec::encode_trade_item_request` / `encode_close_trade`; opcodes in `protocol_opcodes::server`; golden in `protocol_compat` v772.
- `game_loop.rs` — dispatch `0x7D`–`0x80`; trade packets ungated from timed-action drop.
- `house:startTrade` → `LuaMutation::HouseStartTrade` + dual document counter-offer; `!sellhouse` unblocked when mutation scope is live.
- Tests: 9× `trade::tests`, house cancel codes, protocol goldens.

## Verify

```
rtk cargo test -p tfs-rust-core --lib trade
rtk cargo test -p tfs-rust-net --test protocol_compat trade
rtk cargo test -p tfs-rust-lua --lib userdata::house
rtk cargo check --workspace
rtk cargo clippy --workspace --all-targets
```
