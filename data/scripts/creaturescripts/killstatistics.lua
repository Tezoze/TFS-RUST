if not KILL_STATISTICS then
	KILL_STATISTICS = {}
end

local creatureevent = CreatureEvent("KillStatistics_KillMonster")

-- Store information for player killing monsters
function creatureevent.onKill(creature, target)
	if target:getMaster() then
		return true -- Killed creature is a summon
	end

	local name = target:getName()
	if target:getPlayer() then
		name = "players" -- Killed creature is a player
	end

	if name == "Demon" then -- Special check for Demon "Illusion"
		local maxhp = target:getMaxHealth()
		if maxhp <= 50 then
			name = "Illusion"
		end
	end

	local record = KILL_STATISTICS[name]
	if record then
		KILL_STATISTICS[name].killed = KILL_STATISTICS[name].killed + 1
	else
		KILL_STATISTICS[name] = {killed = 1, killedBy = 0}
	end

	return true
end

creatureevent:register()

creatureevent = CreatureEvent("KillStatistics_KillPlayer")

-- Store information for monsters killing players
function creatureevent.onDeath(creature, corpse, killer, mostDamageKiller, lastHitUnjustified, mostDamageUnjustified)
	local name = ""

	if not killer then
		name = "elemental forces"
	elseif killer:getPlayer() then
		name = "players"
	elseif killer:getMaster() then
		local master = killer:getMaster()
		if master:getPlayer() then
			name = "players"
		else
			name = master:getName()
		end
	else
		name = killer:getName()
	end

	if name == "Demon" then
		local maxhp = killer:getMaxHealth()
		if maxhp <= 50 then
			name = "Illusion"
		end
	end

	local record = KILL_STATISTICS[name]
	if record then
		KILL_STATISTICS[name].killedBy = KILL_STATISTICS[name].killedBy + 1
	else
		KILL_STATISTICS[name] = {killed = 0, killedBy = 1}
	end

	return true
end

creatureevent:register()

creatureevent = CreatureEvent("KillStatistics_PlayerLogin")

function creatureevent.onLogin(player)
	player:registerEvent("KillStatistics_KillMonster")
	player:registerEvent("KillStatistics_KillPlayer")
	return true
end

creatureevent:register()
