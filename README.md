TFS Rust [![Build Status](https://github.com/Tezoze/TFS-RUST/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/Tezoze/TFS-RUST/actions/workflows/rust.yml "Rust build status")
===============

TFS Rust is a ground-up rewrite of the Australis TFS 1.4.2 C++ game server into idiomatic Rust. It replaces the legacy codebase with a modern asynchronous architecture while preserving 100% Tibia 8.6 protocol compatibility, 100% Lua API compatibility, and full database schema compatibility. To connect to the server, you should use a custom OTClientv8.

### Engine Architecture

* **Memory Safety & True Concurrency:** Eliminates traditional C++ crashes by utilizing a generational arena (`slotmap::SlotMap`) for entity storage. Network I/O is completely decoupled from the single-threaded game state using the Tokio async runtime and `mpsc` channels.
* **Database Integrity:** Built on async MariaDB access via `sqlx` using only prepared statements, structurally eliminating SQL injection risks.
* **Scripting Bridge:** Features a highly isolated LuaJIT scripting bridge via `mlua`. Script errors are isolated, caught, and logged, allowing the game tick to continue.

### Getting Started

* [Compiling](https://github.com/Tezoze/TFS-RUST/wiki/Compiling), alternatively download [releases](https://github.com/Tezoze/TFS-RUST/releases)
* [Scripting Reference](https://github.com/Tezoze/TFS-RUST/wiki/Script-Interface)
* [Contributing](https://github.com/Tezoze/TFS-RUST/wiki/Contributing)

### Support

If you need help, please visit the [support forum on OTLand](https://otland.net/forums/support.16/). Our issue tracker is not a support forum, and using it as one will result in your issue being closed. If you were unable to get assistance in the support forum, you should consider [becoming a premium user on OTLand](https://otland.net/account/upgrades) which grants you access to the premium support forum and supports OTLand financially.

### Issues

We use the [issue tracker on GitHub](https://github.com/Tezoze/TFS-RUST/issues). Keep in mind that everyone who is watching the repository gets notified by e-mail when there is activity, so be thoughtful and avoid writing comments that aren't meaningful for an issue (e.g. "+1"). If you'd like for an issue to be fixed faster, you should either fix it yourself and submit a pull request, or place a bounty on the issue.
