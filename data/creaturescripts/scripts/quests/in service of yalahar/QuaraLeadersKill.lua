local quaraLeaders

function onKill(creature, target)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- Define storage mapping here when Storage is available
	if not quaraLeaders then
		quaraLeaders = {
			['inky'] = Storage.InServiceofYalahar.QuaraInky,
			['sharptooth'] = Storage.InServiceofYalahar.QuaraSharptooth,
			['splasher'] = Storage.InServiceofYalahar.QuaraSplasher
		}
	end

	-- Check if target is a player or has a master (summoned creature)
	if target:isPlayer() or target:getMaster() then
		return true
	end

	-- Check if player is on the correct quest stage (sunken quarter mission)
	local questline = player:getStorageValue(Storage.InServiceofYalahar.Questline) or 0
	if questline < 40 or questline > 41 then
		return true
	end

	local targetName = target:getName():lower()
	local bossStorage = quaraLeaders[targetName]
	if not bossStorage then
		return true
	end

	-- Only set storage if not already killed
	if (player:getStorageValue(bossStorage) or 0) < 1 then
		player:setStorageValue(bossStorage, 1)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have slain ' .. target:getName() .. '!')
	end

	-- Check if all three quara leaders have been killed
	local quaraInky = player:getStorageValue(Storage.InServiceofYalahar.QuaraInky) or 0
	local quaraSharptooth = player:getStorageValue(Storage.InServiceofYalahar.QuaraSharptooth) or 0
	local quaraSplasher = player:getStorageValue(Storage.InServiceofYalahar.QuaraSplasher) or 0

	if quaraInky == 1 and quaraSharptooth == 1 and quaraSplasher == 1 then
		-- All quara leaders killed - update quest progress
		if player:getStorageValue(Storage.InServiceofYalahar.QuaraState) ~= 2 then
			player:setStorageValue(Storage.InServiceofYalahar.QuaraState, 2)
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have eliminated all quara leaders! Report back to your contact.')
		end
	end

	return true
end
