function onKill(creature, target)
	if not target:isMonster() then
		return true
	end

	local targetMonster = target

	if targetMonster:getName():lower() == 'the keeper' then
		Game.setStorageValue(Storage.WrathoftheEmperor.Mission03, 0)
	end
	return true
end
