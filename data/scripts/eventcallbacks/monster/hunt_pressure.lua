-- Bench hunt pressure — live monster id index.
-- Pack surface: EventCallback onSpawn (`events.cpp` / Rust `monster_spawn.rs`).
-- Store numeric ids only. Do not stash Monster userdata (spawn-scoped inventory).

HuntPressure = HuntPressure or {}
HuntPressure.ids = HuntPressure.ids or {}

local ec = EventCallback
ec.onSpawn = function(monster, _position, _startup, _artificial)
	if not monster then
		return true
	end
	local id = monster:getId()
	if not id or id == 0 then
		return true
	end
	local creature = Creature(id)
	if not creature or not creature:isMonster() then
		return true
	end
	if creature:getMaster() then
		return true
	end
	local ids = HuntPressure.ids
	ids[#ids + 1] = id
	return true
end
ec:register()
