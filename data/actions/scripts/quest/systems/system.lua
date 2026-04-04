local specialQuests = {
	[2215] = Storage.AnnihilatorDone,
	[2016] = Storage.DreamersChallenge.Reward,
	[10544] = Storage.PitsOfInferno.WeaponReward,
	[12513] = Storage.thievesGuild.Reward,
	[12374] = Storage.WrathoftheEmperor.mainReward,
	[26300] = Storage.SvargrondArena.RewardGreenhorn,
	[27300] = Storage.SvargrondArena.RewardScrapper,
	[28300] = Storage.SvargrondArena.RewardWarlord,
	[2000] = Storage.QuestChests.WhisperMoss
}

local questsExperience = {
	[2217] = 1 -- dummy values
}

local questLog = {
	[9130] = Storage.hiddenCityOfBeregar.DefaultStart
}

local tutorialIds = {
	[50080] = 5,
	[50082] = 6,
	[50084] = 10,
	[50086] = 11
}

local achievementChest = {
	-- [item.actionid] = "Achievement name",
	-- Annihilator sample
	[2215] = "Annihilator",
}

local hotaQuest = {12102, 12103, 12104, 12105, 12106, 12107}

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local storage
	-- Prefer UID-based storage for chests with valid UIDs (<= 65535) to allow unique storage per chest
	-- Exception: POH weapon rewards must use shared storage to prevent multiple reward selection
	if item.uid <= 65535 and item.actionid ~= 10544 then
		storage = item.uid
	else
		-- Fall back to actionid-based storage if UID is invalid or for POH weapon rewards
		storage = specialQuests[item.actionid]
		if not storage then
			return false
		end
	end

	-- Special case for lizard scroll box (uid 50034)
	if item.uid == 50034 then
		if (player:getStorageValue(Storage.TheApeCity.ChorDoor) or 0) == 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The box seems to be sealed against unwanted intruders.')
			return true
		end
		storage = item.uid  -- Use uid as storage for one-time use
	end

	-- Special case for Dragha's spellbook collection (uid 50148)
	if item.uid == 50148 then
		storage = item.uid  -- Use uid as storage for one-time use
	end

	-- Special case for Explorer Society family brooch box (uid 9898)
	if item.uid == 9898 then
		if (player:getStorageValue(Storage.ExplorerSociety.QuestLine) or 0) < 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The box seems to be sealed against unwanted intruders.')
			return true
		end
		storage = item.uid  -- Use uid as storage for one-time use
	end

	-- Special case for Palimuth's research notes box (uid 12272)
	if item.uid == 12272 then
		if (player:getStorageValue(Storage.InServiceofYalahar.Mission03) or 0) < 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The box seems to be sealed against unwanted intruders.')
			return true
		end
		storage = item.uid  -- Use uid as storage for one-time use
	end
	
	if storage == Storage.SvargrondArena.RewardGreenhorn or storage == Storage.SvargrondArena.RewardScrapper or storage == Storage.SvargrondArena.RewardWarlord then
		player:setStorageValue(Storage.SvargrondArena.Pit, -1)
	end

	-- Special case for Svargrond Arena Greenhorn reward chests
	if item.uid == 4030 or item.uid == 4031 or item.uid == 4032 then
		if (player:getStorageValue(Storage.SvargrondArena.RewardGreenhorn) or 0) ~= 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have not completed the Greenhorn arena yet.')
			return true
		end

		-- Check if player has already chosen a Greenhorn reward
		if (player:getStorageValue(Storage.SvargrondArena.RewardChosenGreenhorn) or 0) == 2 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have already chosen your Greenhorn reward.')
			return true
		end

		if (player:getStorageValue(storage) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end

		local rewardItemId
		if item.uid == 4030 then
			rewardItemId = 7392
		elseif item.uid == 4031 then
			rewardItemId = 7380
		elseif item.uid == 4032 then
			rewardItemId = 7406
		end

		local reward = Game.createItem(rewardItemId, 1)
		if player:addItemEx(reward) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have earned a reward, but you have no room to take it.')
			return true
		end

		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found a ' .. reward:getName() .. '.')
		player:setStorageValue(storage, 1) -- Mark this specific chest as used
		player:setStorageValue(Storage.SvargrondArena.RewardChosenGreenhorn, 2) -- Mark that player has chosen a Greenhorn reward
		return true
	end

	-- Special case for Svargrond Arena Scrapper reward chests
	if item.uid == 4035 or item.uid == 4036 or item.uid == 4037 then
		if (player:getStorageValue(Storage.SvargrondArena.RewardScrapper) or 0) ~= 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have not completed the Scrapper arena yet.')
			return true
		end

		-- Check if player has already chosen a Scrapper reward
		if (player:getStorageValue(Storage.SvargrondArena.RewardChosenScrapper) or 0) == 2 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have already chosen your Scrapper reward.')
			return true
		end

		if (player:getStorageValue(storage) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end

		local rewardItemId
		if item.uid == 4035 then
			rewardItemId = 7415
		elseif item.uid == 4036 then
			rewardItemId = 7389
		elseif item.uid == 4037 then
			rewardItemId = 7384
		end

		local reward = Game.createItem(rewardItemId, 1)
		if player:addItemEx(reward) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have earned a reward, but you have no room to take it.')
			return true
		end

		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found a ' .. reward:getName() .. '.')
		player:setStorageValue(storage, 1) -- Mark this specific chest as used
		player:setStorageValue(Storage.SvargrondArena.RewardChosenScrapper, 2) -- Mark that player has chosen a Scrapper reward
		return true
	end

	-- Special case for Svargrond Arena Warlord reward chests
	if item.uid == 4040 or item.uid == 4041 or item.uid == 4042 then
		if (player:getStorageValue(Storage.SvargrondArena.RewardWarlord) or 0) ~= 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have not completed the Warlord arena yet.')
			return true
		end

		-- Check if player has already chosen a Warlord reward
		if (player:getStorageValue(Storage.SvargrondArena.RewardChosenWarlord) or 0) == 2 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have already chosen your Warlord reward.')
			return true
		end

		if (player:getStorageValue(storage) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end

		local rewardItemId
		if item.uid == 4040 then
			rewardItemId = 7429
		elseif item.uid == 4041 then
			rewardItemId = 7434
		elseif item.uid == 4042 then
			rewardItemId = 7390
		end

		local reward = Game.createItem(rewardItemId, 1)
		if player:addItemEx(reward) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have earned a reward, but you have no room to take it.')
			return true
		end

		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found a ' .. reward:getName() .. '.')
		player:setStorageValue(storage, 1) -- Mark this specific chest as used
		player:setStorageValue(Storage.SvargrondArena.RewardChosenWarlord, 2) -- Mark that player has chosen a Warlord reward
		return true
	end

	-- Special case for lizard scroll box
	if item.uid == 50034 then
		if (player:getStorageValue(storage) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end
		local scroll = Game.createItem(5956, 1)
		if player:addItemEx(scroll) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have found a lizard scroll, but you have no room to take it.')
			return true
		end
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found a lizard scroll.')
		player:setStorageValue(storage, 1)
		return true
	end

	-- Special case for Explorer Society family brooch box
	if item.uid == 9898 then
		if (player:getStorageValue(storage) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end
		local brooch = Game.createItem(4845, 1) -- Dwarven family brooch from Uzgod quest
		if player:addItemEx(brooch) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have found a family brooch, but you have no room to take it.')
			return true
		end
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found a family brooch.')
		player:setStorageValue(storage, 1)
		return true
	end

	-- Special case for Palimuth's research notes box
	if item.uid == 12272 then
		if (player:getStorageValue(Storage.InServiceofYalahar.ResearchNotesBox) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end
		local notes = Game.createItem(9680, 1) -- Research notes
		notes:setActionId(30219) -- Set actionid for reading the notes
		if player:addItemEx(notes) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have found research notes, but you have no room to take them.')
			return true
		end
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found some research notes.')
		player:setStorageValue(Storage.InServiceofYalahar.ResearchNotesBox, 1)
		return true
	end

	-- Special case for Dragha's spellbook collection
	if item.uid == 50148 then
		if (player:getStorageValue(storage) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end
		local spellbook = Game.createItem(6120, 1) -- Dragha's spellbook
		if player:addItemEx(spellbook) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have found Dragha\'s spellbook, but you have no room to take it.')
			return true
		end
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found Dragha\'s spellbook.')
		player:setStorageValue(storage, 1)
		player:setStorageValue(Storage.TheShatteredIsles.DragahsSpellbook, 0) -- Allow delivery to Chondur
		return true
	end

	-- Special case for Azerus manifesto box
	if item.uid == 12273 then
		if (player:getStorageValue(Storage.InServiceofYalahar.Questline) or 0) < 18 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The box seems to be sealed against unwanted intruders.')
			return true
		end
		if (player:getStorageValue(Storage.InServiceofYalahar.AzerusNotesBox) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end
		local manifesto = Game.createItem(9680, 1) -- Yalahari manifesto
		manifesto:setActionId(30218) -- Set actionid for reading the manifesto
		if player:addItemEx(manifesto) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have found a yalahari manifesto, but you have no room to take it.')
			return true
		end
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found a yalahari manifesto.')
		player:setStorageValue(Storage.InServiceofYalahar.AzerusNotesBox, 1)
		return true
	end

	-- Special case for Secret Service parchment box (Mission 2)
	if item.uid == 12570 then
		if (player:getStorageValue(Storage.secretService.TBIMission02) or 0) < 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The box seems to be sealed against unwanted intruders.')
			return true
		end
		if (player:getStorageValue(12589) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end
		local parchment = Game.createItem(7696, 1) -- Parchment for Chester's Mission 2
		if player:addItemEx(parchment) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have found a parchment, but you have no room to take it.')
			return true
		end
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found a parchment.')
		player:setStorageValue(12589, 1) -- Mark box as used
		return true
	end

	-- Special case for Secret Service scroll chest (Mission 3)
	if item.uid == 12571 then
		if (player:getStorageValue(Storage.secretService.TBIMission03) or 0) < 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The chest seems to be sealed against unwanted intruders.')
			return true
		end
		if (player:getStorageValue(12590) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end
		local scroll = Game.createItem(14324, 1) -- Scroll for Chester's Mission 3
		if player:addItemEx(scroll) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have found a scroll, but you have no room to take it.')
			return true
		end
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found a scroll.')
		player:setStorageValue(12590, 1) -- Mark chest as used
		return true
	end

	-- Special case for Secret Service intelligence reports chest (Mission 4)
	if item.uid == 12572 then
		if (player:getStorageValue(Storage.secretService.TBIMission04) or 0) < 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The corpse contains some sort of intelligence report, you have no use for it.')
			return true
		end
		if (player:getStorageValue(12591) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end
		local reports = Game.createItem(14325, 1) -- Intelligence reports for Chester's Mission 4
		if player:addItemEx(reports) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have found intelligence reports, but you have no room to take them.')
			return true
		end
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found some intelligence reports.')
		player:setStorageValue(12591, 1) -- Mark chest as used
		return true
	end

	-- Special case for Whisper Moss box
	if item.uid == 50033 then
		if (player:getStorageValue(Storage.QuestChests.WhisperMoss) or 0) > 0 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
			return true
		end
		local moss = Game.createItem(4838, 1) -- Whisper moss
		if player:addItemEx(moss) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have found whisper moss, but you have no room to take it.')
			return true
		end
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found whisper moss.')
		player:setStorageValue(Storage.QuestChests.WhisperMoss, 1) -- Mark box as used
		return true
	end

	-- Special case for Annihilator reward chests (pick 1 of 4)
	if item.uid == 4015 or item.uid == 4016 or item.uid == 4017 or item.uid == 4018 then
		if (player:getStorageValue(Storage.AnnihilatorDone) or 0) ~= 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have not completed the Annihilator quest yet.')
			return true
		end

		if (player:getStorageValue(Storage.AnnihilatorRewardChosen) or 0) == 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have already chosen your Annihilator reward.')
			return true
		end

		local rewardItemId
		if item.uid == 4015 then
			rewardItemId = 2494 -- Demon Armor
		elseif item.uid == 4016 then
			rewardItemId = 2400 -- Magic Sword
		elseif item.uid == 4017 then
			rewardItemId = 2431 -- Stonecutter's Axe
		elseif item.uid == 4018 then
			-- Present box with teddy bear inside
			local presentBox = Game.createItem(1990, 1)
			local teddyBear = Game.createItem(2326, 1)
			presentBox:addItemEx(teddyBear)
			if player:addItemEx(presentBox) ~= RETURNVALUE_NOERROR then
				player:sendCancelMessage('You have earned a reward, but you have no room to take it.')
				return true
			end
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found a present box.')
			player:setStorageValue(Storage.AnnihilatorRewardChosen, 1)
			player:addAchievement("Annihilator")
			return true
		end

		local reward = Game.createItem(rewardItemId, 1)
		if player:addItemEx(reward) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have earned a reward, but you have no room to take it.')
			return true
		end

		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found a ' .. reward:getName() .. '.')
		player:setStorageValue(Storage.AnnihilatorRewardChosen, 1)
		player:addAchievement("Annihilator")
		return true
	end

	-- Special case for Suspicious Documents chest (Emma Mission 5)
	if item.uid == 12581 then
		if (player:getStorageValue(Storage.secretService.CGBMission05) or 0) < 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The chest seems to be sealed against unwanted intruders.')
			return true
		end
		local documents = Game.createItem(7703, 1) -- Suspicious documents
		if player:addItemEx(documents) ~= RETURNVALUE_NOERROR then
			player:sendCancelMessage('You have found suspicious documents, but you have no room to take them.')
			return true
		end
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found some suspicious documents.')
		player:setStorageValue(12592, 1) -- Mark chest as used (avoiding conflicts)
		return true
	end

	if (player:getStorageValue(storage) or 0) > 0 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The ' .. ItemType(item.itemid):getName() .. ' is empty.')
		return true
	end

	local items, reward = {}
	local size = item:isContainer() and item:getSize() or 0
	if size == 0 then
		reward = item:clone()
	else
		local container = Container(item.uid)
		for i = 0, container:getSize() - 1 do
			items[#items + 1] = container:getItem(i):clone()
		end
	end

	size = #items
	if size == 1 then
		reward = items[1]:clone()
	end

	local result = ''
	if reward then
		local ret = ItemType(reward.itemid)
		if ret:isRune() then
			result = ret:getArticle() .. ' ' ..  ret:getName() .. ' (' .. reward.type .. ' charges)'
		elseif ret:isStackable() and reward:getCount() > 1 then
			result = reward:getCount() .. ' ' .. ret:getPluralName()
		elseif ret:getArticle() ~= '' then
			result = ret:getArticle() .. ' ' .. ret:getName()
		else
			result = ret:getName()
		end
	else
		if size > 20 then
			reward = Game.createItem(item.itemid, 1)
		elseif size > 8 then
			reward = Game.createItem(1988, 1)
		else
			reward = Game.createItem(1987, 1)
		end

		for i = 1, size do
			local tmp = items[i]
			if reward:addItemEx(tmp) ~= RETURNVALUE_NOERROR then
				print('[Warning] QuestSystem:', 'Could not add quest reward to container')
			end
		end
		local ret = ItemType(reward.itemid)
		result = ret:getArticle() .. ' ' .. ret:getName()
	end

	if player:addItemEx(reward) ~= RETURNVALUE_NOERROR then
		local weight = reward:getWeight()
		if player:getFreeCapacity() < weight then
			player:sendCancelMessage(string.format('You have found %s weighing %.2f oz. You have no capacity.', result, (weight / 100)))
		else
			player:sendCancelMessage('You have found ' .. result .. ', but you have no room to take it.')
		end
		return true
	end

	if questsExperience[storage] then
		player:addExperience(questsExperience[storage], true)
	end

	if questLog[storage] then
		player:setStorageValue(questLog[storage], 1)
	end

	if tutorialIds[storage] then
		-- Special handling for deeper Banuta forcefields
		if storage == 50085 or storage == 50086 then
			if player:getStorageValue(Storage.DeeperBanutaShortcut) ~= 1 then
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You need to use the egg of The Many on the giant lizard head first.")
				return true
			end
		end

		player:sendTutorial(tutorialIds[storage])
		if item.uid == 50080 then
			player:setStorageValue(Storage.RookgaardTutorialIsland.SantiagoNpcGreetStorage, 3)
		end
	end

	if isInArray(hotaQuest, item.uid) then
		if player:getStorageValue(Storage.TheAncientTombs.DefaultStart) ~= 1 then
			player:setStorageValue(Storage.TheAncientTombs.DefaultStart, 1)
		end
	end

	-- add Achievements
	if(achievementChest[item.actionid])then
		player:addAchievement(achievementChest[item.actionid])
	end

	player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have found ' .. result .. '.')
	player:setStorageValue(storage, 1)
	return true
end
