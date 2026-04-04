dofile('data/lib/core/storages.lua')

function onKill(player, target)
	if not target:isMonster() or target:getName():lower() ~= 'ungreez' then
		return true
	end

	local questline = player:getStorageValue(12160) or 0 -- Storage.TheInquisition.Questline
	local mission06 = player:getStorageValue(12166) or 0 -- Storage.TheInquisition.Mission06

	if questline == 18 and mission06 < 2 then
		-- The Inquisition Questlog- 'Mission 6: The Demon Ungreez'
		player:setStorageValue(12166, 2) -- Storage.TheInquisition.Mission06
		player:setStorageValue(12160, 19) -- Storage.TheInquisition.Questline
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have slain Ungreez! Return to Henricus for your reward.')
		-- Game.sendQuestUpdateMessage(player) -- Function not available in this TFS version
	end
	return true
end
