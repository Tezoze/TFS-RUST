function onKill(player, target)
	if not target:isMonster() then
		return true
	end

	if target:getName():lower() ~= 'black knight' then
		return true
	end

	if player:getStorageValue(Storage.secretService.AVINMission04) == 1 then
		player:setStorageValue(Storage.secretService.AVINMission04, 2)
		player:setStorageValue(12561, 2) -- Mission 4: Black Knight killed
	end

	return true
end
