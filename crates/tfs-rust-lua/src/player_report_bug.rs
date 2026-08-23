//! Dispatch `EventCallback(EVENT_CALLBACK_ONREPORTBUG)`.
//!
//! Pack: TFS `Events::eventPlayerOnReportBug` (`events.cpp`) via allowlisted
//! `data/scripts/eventcallbacks/player/default_onReportBug.lua`.
//! Wire: `GamePacket::BugReport` (previously discarded).

use mlua::{ObjectLike, Table, Value};

use crate::context::CreatureRef;
use crate::event_callback::EVENT_CALLBACK_ONREPORTBUG;
use crate::runtime::{LuaError, LuaRuntime};
use crate::userdata::position::PositionRef;

impl LuaRuntime {
    /// `EventCallback(ONREPORTBUG, player, message, position, category)`.
    pub fn call_player_on_report_bug(
        &self,
        player: u64,
        message: &str,
        x: u16,
        y: u16,
        z: u8,
        category: u8,
    ) -> Result<(), LuaError> {
        if !self.has_event_callback(EVENT_CALLBACK_ONREPORTBUG) {
            return Ok(());
        }
        let globals = self.lua.globals();
        let Ok(ec) = globals.get::<Table>("EventCallback") else {
            return Ok(());
        };
        let player_ud = self.lua.create_userdata(CreatureRef(player))?;
        let pos = self.lua.create_userdata(PositionRef { x, y, z })?;
        let _: Value = ec.call((
            EVENT_CALLBACK_ONREPORTBUG,
            player_ud,
            message,
            pos,
            category,
        ))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::load_data_lib;
    use std::path::PathBuf;

    fn workspace_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    #[test]
    fn report_bug_runs_when_allowlisted() {
        let data_root = workspace_data_root();
        if !data_root.join("scripts/lib/event_callbacks.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }
        let runtime = LuaRuntime::new().expect("runtime");
        load_data_lib(&runtime, &data_root).expect("data lib");
        {
            let _guard = runtime.enter_scripts_interface();
            runtime
                .exec_chunk(
                    "test_on_report_bug",
                    r#"
                    local ec = EventCallback
                    ec.onReportBug = function(player, message, position, category)
                        REPORT_BUG_SEEN = message .. ":" .. tostring(category)
                        return true
                    end
                    ec:register()
                    "#,
                )
                .expect("register onReportBug");
        }
        assert!(runtime.has_event_callback(EVENT_CALLBACK_ONREPORTBUG));
        runtime
            .call_player_on_report_bug(1, "test report", 100, 100, 7, 2)
            .expect("report-bug dispatch");
        runtime
            .exec_chunk(
                "assert_report_bug",
                "assert(REPORT_BUG_SEEN == 'test report:2', tostring(REPORT_BUG_SEEN))",
            )
            .expect("payload reached callback");
    }
}
