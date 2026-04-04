local bosses = {
	['ushuriel'] = 200,    -- Global storage for room access
	['zugurosh'] = 201,
	['madareth'] = 202,
	['latrivan'] = 203,
	['golgordan'] = 203,
	['annihilon'] = 204,
	['hellgorak'] = 205
}

function onKill(player, target)
	if not target:isMonster() then
		return true
	end

	local targetName = target:getName():lower()
	local bossStorage = bosses[targetName]
	if not bossStorage then
		return true
	end

	local newValue = 2
	if targetName == 'latrivan' or targetName == 'golgordan' then
		newValue = math.max(0, Game.getStorageValue(bossStorage) or 0) + 1
	end
	Game.setStorageValue(bossStorage, newValue)

	if newValue == 2 then
		local spectators = Game.getSpectators(player:getPosition(), false, true, 7, 7)
		for i = 1, #spectators do
			local spectator = spectators[i]
			if spectator:isPlayer() then
				spectator:say('You now have 10 minutes to exit this room through the teleporter. It will bring you to the next room.', TALKTYPE_MONSTER_SAY)
			end
		end
		addEvent(Game.setStorageValue, 10 * 60 * 1000, bossStorage, 0)
	end
	return true
end
