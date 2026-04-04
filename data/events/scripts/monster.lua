-- Rarity Animations
local rare_popup = true
local rare_effect = true
local rare_effect_id = CONST_ME_FIREWORK_YELLOW

function Monster:onDropLoot(corpse)
	if configManager.getNumber(configKeys.RATE_LOOT) == 0 then
		return
	end

--	if hasEventCallback(EVENT_CALLBACK_ONDROPLOOT) then
--		EventCallback(EVENT_CALLBACK_ONDROPLOOT, self, corpse)
--	end

	local player = Player(corpse:getCorpseOwner())
	local mType = self:getType()

	if not player or player:getStamina() > 840 then
		local monsterLoot = mType:getLoot()
		for i = 1, #monsterLoot do
			local item = corpse:createLootItem(monsterLoot[i])
			if not item then
				print('[Warning] DropLoot:', 'Could not add loot item to corpse.')
			end
		end

		-- Server-side autoloot: move matching items from corpse to player inventory
		if player and doAutoLoot then
			doAutoLoot(player, corpse)
		end

		if player then
			local text = ("Loot of %s: %s"):format(mType:getNameDescription(), corpse:getContentDescription())
			local party = player:getParty()
			if party then
				party:broadcastPartyLoot(text)
			else
				player:sendTextMessage(MESSAGE_LOOT, text)
			end
		end
	else
		local text = ("Loot of %s: nothing (due to low stamina)"):format(mType:getNameDescription())
		local party = player:getParty()
		if party then
			party:broadcastPartyLoot(text)
		else
			player:sendTextMessage(MESSAGE_LOOT, text)
		end
	end

	-- Send kill tracker to player (forgottenserver pattern) - DISABLED
	-- if player then
	-- 	player:updateKillTracker(self, corpse)
	-- end

	-- Apply rarity chance to corpse contents and apply animation
	local rarityResult, rollInfo = rollRarity(corpse)
	if rarityResult > 0 then -- If a rare item was rolled, play animation
		if rare_popup then
			local spectators = Game.getSpectators(corpse:getPosition(), false, true, 7, 7, 5, 5)
			for i = 1, #spectators do
				-- Get the rarity text from the global rollRarity function result
				local rare_text = "Rare!"
				if rarityResult == 2 then
					rare_text = "Epic!"
				elseif rarityResult == 3 then
					rare_text = "Legendary!"
				elseif rarityResult == 4 then
					rare_text = "Mythic!"
				end
				spectators[i]:say(rare_text, TALKTYPE_MONSTER_SAY, false, spectators[i], corpse:getPosition())
			end
		end
		if rare_effect then
			local pos = corpse:getPosition()
			pos:sendMagicEffect(rare_effect_id)
			for i = 1, 3 do
				addEvent(function(p) Position(p):sendMagicEffect(rare_effect_id) end, i * 350, {x = pos.x, y = pos.y, z = pos.z})
			end
		end
		-- Send server log notification to the killing player (light blue message)
		if player and rollInfo and #rollInfo > 0 then
			for _, info in ipairs(rollInfo) do
				local msg = string.format("[%s Drop] %s %s", info.tier, info.itemName, info.stats)
				player:sendTextMessage(MESSAGE_INFO_DESCR, msg)
			end
		end
	end
end

function Monster:onSpawn(position, startup, artificial)
    self:registerEvent("rollHealth")
    self:registerEvent("rollMana")
    self:registerEvent("TaskDamageBoost")
	if hasEventCallback(EVENT_CALLBACK_ONSPAWN) then
		return EventCallback(EVENT_CALLBACK_ONSPAWN, self, position, startup, artificial)
	else
		return true
	end
end

