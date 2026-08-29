# Immediate logout on 772 login

**Status:** complete.
**Symptom:** Login registers, then ~150ms later `LOGOUT` then `game connection closed`. Second Enter Game often sticks.

## Cause

`last_command_round` / `last_action_round` start at 0 (`login.rs`). `ProcessConnections` computes `round_nr - last_command_round`. After ~90 Other rounds of uptime that is already a dead connection (`connections.cc:37`). Next Other tick (often the first beat after login) kicks the player before the client can send a command.

Second attempt can survive if a ping/move stamps the rounds before the next Other fire (~1s).

## Fix

`register_conn_mapping` calls `player_reset_connection_rounds(..., true)` so attach stamps `LastCommand`/`LastAction` to current `RoundNr`.
