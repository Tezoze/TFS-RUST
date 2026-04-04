local diseasedTrio

function onKill(creature, target)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- Define storage mapping here when Storage is available
	if not diseasedTrio then
		diseasedTrio = {
			['diseased bill'] = Storage.InServiceofYalahar.DiseasedBill,
			['diseased dan']  = Storage.InServiceofYalahar.DiseasedDan,
			['diseased fred'] = Storage.InServiceofYalahar.DiseasedFred
		}
	end

	-- Check if target is a player or has a master (summoned creature)
	if target:isPlayer() or target:getMaster() then
		return true
	end

	-- Check if player is on the correct quest stage (alchemist quarter mission)
	local questline = player:getStorageValue(Storage.InServiceofYalahar.Questline) or 0
	if questline < 20 or questline > 21 then
		return true
	end

	local targetName = target:getName():lower()
	local bossStorage = diseasedTrio[targetName]
	if not bossStorage then
		return true
	end

	-- Only set storage if not already killed
	if (player:getStorageValue(bossStorage) or 0) < 1 then
		player:setStorageValue(bossStorage, 1)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have slain ' .. target:getName() .. '!')
	end

	-- Check if all three diseased creatures have been killed
	local diseasedBill = player:getStorageValue(Storage.InServiceofYalahar.DiseasedBill) or 0
	local diseasedDan = player:getStorageValue(Storage.InServiceofYalahar.DiseasedDan) or 0
	local diseasedFred = player:getStorageValue(Storage.InServiceofYalahar.DiseasedFred) or 0

	if diseasedBill == 1 and diseasedDan == 1 and diseasedFred == 1 then
		-- All diseased creatures killed - enable formula collection
		if player:getStorageValue(Storage.InServiceofYalahar.AlchemistFormula) ~= 1 then
			player:setStorageValue(Storage.InServiceofYalahar.AlchemistFormula, 0)
			player:setStorageValue(12258, 1) -- Access to formula collection area door
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have eliminated all plague carriers! Now find and retrieve the alchemist formulas.')
		end
	end
	
	return true
end
