local config = {
	amphoraPositions = {
		Position(32792, 32527, 10),
		Position(32823, 32525, 10),
		Position(32876, 32584, 10),
		Position(32744, 32586, 10)
	},
	remainsId = 4997  -- remains of a canopic jar
}

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Check if player has started the final mission first
	if player:getStorageValue(Storage.TheApeCity.Questline) < 17 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The magic forcefield prevents you from passing through.")
		return true
	end

	-- Check if all amphoras have been broken (remains present)
	local allBroken = true
	for i = 1, #config.amphoraPositions do
		local remainsItem = Tile(config.amphoraPositions[i]):getItemById(config.remainsId)
		if not remainsItem then
			allBroken = false
			break
		end
	end

	if allBroken then
		-- Allow passage
		return false
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "There are 4 large amphoras that must be broken in order to pass through the forcefield.")
		return true
	end
end
