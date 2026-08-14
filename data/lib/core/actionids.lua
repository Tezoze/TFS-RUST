-- TFS 1098 extras only. TVP 4000–4005 live in `data/global.lua`.
-- Merge so the `data/lib/core` scan does not replace the TVP table.
actionIds = actionIds or {}
actionIds.levelDoor = actionIds.levelDoor or 1000
actionIds.citizenship = actionIds.citizenship or 30020
actionIds.citizenshipLast = actionIds.citizenshipLast or 30050
