-- Bench hunt pressure — map-wide monster deaths scaled to online count.
-- Dual: same file on Rust and TVP. Not GlobalEvent:interval — Rust does not
-- dispatch :interval / onThink (lesson: globalevents engine owns timers).
-- Startup + addEvent; kill via addHealth(-max) like pack `/killall`.

HuntPressure = HuntPressure or {}
HuntPressure.ids = HuntPressure.ids or {}
HuntPressure.acc = HuntPressure.acc or 0
-- ~30% of online are hunting; ~1 kill / 12 s → 1000 online ≈ 25 deaths/s.
HuntPressure.HUNT_FRAC = 0.30
HuntPressure.KILL_INTERVAL_S = 12
HuntPressure.MAX_PER_TICK = 40

local function swapRemove(ids, idx)
	ids[idx] = ids[#ids]
	ids[#ids] = nil
end

local function huntPressureTick()
	addEvent(huntPressureTick, 1000)
	local players = Game.getPlayers()
	local online = players and #players or 0
	HuntPressure.acc = HuntPressure.acc
		+ (online * HuntPressure.HUNT_FRAC / HuntPressure.KILL_INTERVAL_S)
	local n = math.floor(HuntPressure.acc)
	HuntPressure.acc = HuntPressure.acc - n
	if n > HuntPressure.MAX_PER_TICK then
		n = HuntPressure.MAX_PER_TICK
	end
	local ids = HuntPressure.ids
	if n < 1 or #ids < 1 then
		return
	end
	local killed = 0
	local tries = n * 3
	while killed < n and tries > 0 and #ids > 0 do
		tries = tries - 1
		local idx = math.random(#ids)
		local id = ids[idx]
		local creature = Creature(id)
		if not creature or not creature:isMonster() or creature:getMaster() then
			swapRemove(ids, idx)
		else
			creature:addHealth(-creature:getMaxHealth())
			swapRemove(ids, idx)
			killed = killed + 1
		end
	end
end

local ge = GlobalEvent("HuntPressure")
function ge.onStartup()
	addEvent(huntPressureTick, 1000)
	return true
end
ge:type("startup")
ge:register()
