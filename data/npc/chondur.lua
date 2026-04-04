-- Chondur - Converted from XML to Lua NpcType
-- Original XML: data/npc/Chondur.xml
-- Original Script: data/npc/scripts/Chondur.lua

local npcName = "Chondur"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a chondur")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 154, lookHead = 38, lookBody = 113, lookLegs = 119, lookFeet = 116, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, 'stampor') or msgcontains(msg, 'mount') then
		if not player:hasMount(11) then
				npcHandler:say('You did bring all the items I requqested, cuild. Good. Shall I travel to the spirit realm and try finding a stampor compasion for you?', cid)
				npcHandler.topic[cid] = 1
		else
			npcHandler:say('You already have stampor mount.', cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 1 then
		if player:removeItem(13299, 50) and player:removeItem(13301, 30) and player:removeItem(13300, 100) then
			npcHandler:say({
				'Ohhhhh Mmmmmmmmmmmm Ammmmmgggggggaaaaaaa ...',
				'Aaaaaaaaaahhmmmm Mmmaaaaaaaaaa Kaaaaaamaaaa ...',
				'Brrt! I think it worked! It\'s a male stampor. I linked this spirit to yours. You can probably already summon him to you ...',
				'So, since me are done here... I need to prepare another ritual, so please let me work, cuild.'
			}, cid)
			player:addMount(11)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_RED)
		else
			npcHandler:say('Sorry you don\'t have the necessary items.', cid)
		end
		npcHandler.topic[cid] = 0
	elseif msgcontains(msg, 'no') and npcHandler.topic[cid] > 2 then
		npcHandler:say('Maybe next time.', cid)
		npcHandler.topic[cid] = 0
	elseif msgcontains(msg, 'spellbook') then
		-- Early storage check to block repeats
		if player:getStorageValue(Storage.TheShatteredIsles.DragahsSpellbook) == 1 then
			npcHandler:say("You already delivered Dragha's spellbook to me.", cid)
			npcHandler.topic[cid] = 0
			return true
		end

		-- Check if player has collected the spellbook
		if player:getStorageValue(Storage.TheShatteredIsles.DragahsSpellbook) == 0 then
			-- Player has the spellbook, try to deliver it
			if player:getItemCount(6120) > 0 then
				if player:removeItem(6120, 1) then
					npcHandler:say("Ah, thank you very much! I'll honour his memory.", cid)
					player:setStorageValue(Storage.TheShatteredIsles.DragahsSpellbook, 1)
					player:setStorageValue(Storage.TheShatteredIsles.TheCounterspell, 0)
					npcHandler.topic[cid] = 0
					return true
				else
					npcHandler:say("Something went wrong.", cid)
					npcHandler.topic[cid] = 0
					return true
				end
			else
				npcHandler:say("You don't have Dragha's spellbook.", cid)
				npcHandler.topic[cid] = 0
				return true
			end
		else
			-- Player doesn't know about the spellbook yet
			npcHandler:say("You should not talk about things you don't know anything about.", cid)
			npcHandler.topic[cid] = 0
			return true
		end
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] >= 12 and npcHandler.topic[cid] <= 14 then
		-- Counterspell corpse deliveries
		local currentStage = npcHandler.topic[cid] - 11 -- 12=1, 13=2, 14=3
		local itemId, nextMessage

		if currentStage == 1 then
			itemId = 4265 -- fresh dead chicken
			nextMessage = 'Very good! <mumblemumble> \'Your soul shall be protected!\' Now, I need a fresh dead rat.'
		elseif currentStage == 2 then
			itemId = 2813 -- fresh dead rat
			nextMessage = 'Very good! <chants and dances> \'You shall face black magic without fear!\' Now, I need a fresh dead black sheep.'
		elseif currentStage == 3 then
			itemId = 2914 -- fresh dead black sheep
			nextMessage = 'Very good! <stomps staff on ground> \'EVIL POWERS SHALL NOT KEEP YOU ANYMORE! SO BE IT!\''
		end

		if player:getItemCount(itemId) > 0 then
			player:removeItem(itemId, 1)
			player:setStorageValue(Storage.TheShatteredIsles.TheCounterspell, currentStage + 1)
			npcHandler:say(nextMessage, cid)
		else
			npcHandler:say("You don't have that item.", cid)
		end
		npcHandler.topic[cid] = 0
		return true
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 11 then
		-- Accept counterspell ritual
		npcHandler:say('This is really not advisable. Behind this barrier, strong forces are raging violently. Are you sure that you want to go there?', cid)
		npcHandler.topic[cid] = 15
		return true
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 15 then
		-- Confirm acceptance of counterspell ritual
		npcHandler:say({
			'I guess I cannot stop you then. Since you told me about my apprentice, it\'s my turn to help you. I\'ll perform a ritual for you, but I need a few ingredients. ...',
			'Bring me one fresh dead chicken, one fresh dead rat and one fresh dead black sheep, in that order.'
		}, cid)
		player:setStorageValue(Storage.TheShatteredIsles.TheCounterspell, 1)
		npcHandler.topic[cid] = 0
		return true
	elseif msgcontains(msg, 'no') and npcHandler.topic[cid] >= 11 and npcHandler.topic[cid] <= 15 then
		-- Decline counterspell ritual
		npcHandler:say('It\'s much safer for you to stay here anyway, trust me.', cid)
		npcHandler.topic[cid] = 0
		return true
	end
	return true
end

-- Shaman Addons
-- If the player can't wear shaman outfit
local function notReadyKeyword(keyword, text)
	keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = text}, function(player) return not player:hasOutfit(player:getSex() == PLAYERSEX_FEMALE and 158 or 154) end)
end

notReadyKeyword('outfit', {'Hum? Sorry, but I don\'t sense enough spiritual wisdom in you to even allow you to touch the mask and staff I\'m wearing... yet. ...', 'I know of a really wise ape healer, though, who might be able to bless you with shamanic energy. You should become his apprentice first if you desire to become mine.'})
notReadyKeyword('addon', {'Hum? Sorry, but I don\'t sense enough spiritual wisdom in you to even allow you to touch the mask and staff I\'m wearing... yet. ...', 'I know of a really wise ape healer, though, who might be able to bless you with shamanic energy. You should become his apprentice first if you desire to become mine.'})
notReadyKeyword('task', 'The time hasn\'t come yet, my child. Believe and learn.')

-- Start task
local function addTaskKeyword(text, value, missionStorage)
	local taskKeyword = keywordHandler:addKeyword({'task'}, StdModule.say, {npcHandler = npcHandler, text = text[1]}, function(player) return player:getStorageValue(Storage.OutfitQuest.Shaman.AddonStaffMask) == value end)
		local yesKeyword = taskKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = text[2]})

			yesKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = text[3], reset = true}, nil,
				function(player)
					player:setStorageValue(Storage.OutfitQuest.Shaman.AddonStaffMask, math.max(0, player:getStorageValue(Storage.OutfitQuest.Shaman.AddonStaffMask)) + 1)
					player:setStorageValue(missionStorage, 1)
					player:setStorageValue(Storage.OutfitQuest.Ref, math.max(0, player:getStorageValue(Storage.OutfitQuest.Ref)) + 1) end
				)
			yesKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'Would you like me to repeat the task requirements then?', moveup = 2})

		taskKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'Well, it seems you aren\'t ready yet.', reset = true})
	keywordHandler:addAliasKeyword({'addon'})
	keywordHandler:addAliasKeyword({'outfit'})
end

-- Staff
addTaskKeyword({
		'If you fulfil a task for me, I\'ll grant you a staff like the one I\'m wearing. Do you want to hear the requirements?',
		{
			'Deep in the Tiquandian jungle a monster lurks which is seldom seen. It is the revenge of the jungle against humankind. ...',
			'This monster, if slain, carries a rare root called Mandrake. If you find it, bring it to me. Also, gather 5 of the voodoo dolls used by the mysterious dworc voodoomasters. ...',
			'If you manage to fulfil this task, I will grant you your own staff. Have you understood everything and are ready for this test?'
		},
		'Good! Come back once you\'ve found a mandrake and collected 5 dworcish voodoo dolls.'
	}, -1, Storage.OutfitQuest.Shaman.MissionStaff
)

-- Mask
addTaskKeyword({
		'You have successfully passed the first task. If you can fulfil my second task, I\'ll grant you a mask like the one I\'m wearing. Do you want to hear the requirements?',
		{
			'The dworcs of Tiquanda like to wear certain tribal masks which I\'d like to take a look at. Please bring me 5 of these masks. ...',
			'Secondly, the high ape magicians of Banuta use banana staffs. I\'d love to learn more about theses staffs, so please bring me 5 of them, too. ...',
			'If you manage to fulfil this task, I\'ll grant you your own mask. Have you understood everything and are you ready for this test?'
		},
		'Good! Come back once you have collected 5 tribal masks and 5 banana staffs.'
	}, 2, Storage.OutfitQuest.Shaman.MissionMask
)

-- Hand in task items
local function addItemKeyword(keyword, aliasKeyword, text, value, item, addonId, missionStorage, achievement)
	local itemKeyword = keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = text[1]}, function(player) return player:getStorageValue(Storage.OutfitQuest.Shaman.AddonStaffMask) == value end)
		itemKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = text[2], reset = true}, function(player) return player:getItemCount(item[1].itemId) < item[1].count or player:getItemCount(item[2].itemId) < item[2].count end)

		itemKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = text[3], reset = true},
			function(player) return player:getItemCount(item[1].itemId) >= item[1].count and player:getItemCount(item[2].itemId) >= item[2].count end,
			function(player)
				player:removeItem(item[1].itemId, item[1].count)
				player:removeItem(item[2].itemId, item[2].count)
				player:addOutfitAddon(158, addonId)
				player:addOutfitAddon(154, addonId)
				player:setStorageValue(Storage.OutfitQuest.Shaman.AddonStaffMask, player:getStorageValue(Storage.OutfitQuest.Shaman.AddonStaffMask) + 1)
				player:setStorageValue(Storage.OutfitQuest.Ref, math.min(0, player:getStorageValue(Storage.OutfitQuest.Ref) - 1))
				player:setStorageValue(missionStorage, 0)
				if achievement then
					player:addAchievement('Way of the Shaman')
				end
				player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
			end
		)
		itemKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'Well, it seems you aren\'t ready yet.', reset = true})
	keywordHandler:addAliasKeyword({aliasKeyword})
	keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = aliasKeyword and text[4] or text[3]})
end

addItemKeyword('mandrake', 'voodoo doll',
	{
		'Have you gathered the mandrake and the 5 voodoo dolls from the dworcs?',
		'I\'m proud of you my child, excellent work. This staff shall be yours from now on!',
		'A rare root with mysterious powers.',
		{
			'Together with the spirits of the ancestors, I seek for wisdom. Together we can change the flow of magic to do things that are beyond the limits of ordinary magic. ...',
			'In conversations with the spirits, I gain insight into secrets that would have been lost otherwise.'
		}
	}, 1, {{itemId = 5015, count = 1}, {itemId = 3955, count = 5}}, 2, Storage.OutfitQuest.Shaman.MissionStaff
)
addItemKeyword('tribal mask', 'banana staff',
	{
		'Have you gathered the 5 tribal masks and the 5 banana staffs?',
		'Well done, my child! I hereby grant you the right to wear a shamanic mask. Do it proudly.',
		'Sometimes dworcs are seen with these masks.',
		'A banana staff is the sign of a high ape magician.'
	}, 3, {{itemId = 3966, count = 5}, {itemId = 3967, count = 5}}, 1, Storage.OutfitQuest.Shaman.MissionMask, true
)

-- Task status
local function addTaskStatusKeyword(keyword, text, value)
	keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = text}, function(player) return player:getStorageValue(Storage.OutfitQuest.Shaman.AddonStaffMask) == value end)
	if keyword == 'addon' then
		keywordHandler:addAliasKeyword({'outfit'})
	end
end

addTaskStatusKeyword('task', 'Your task is to retrieve a mandrake from the Tiquandan jungle and 5 dworcish voodoo dolls.', 1)
addTaskStatusKeyword('task', 'Your task is to retrieve 5 tribal masks from the dworcs and 5 banana staffs from the apes.', 3)
addTaskStatusKeyword('task', 'You have successfully passed all of my tasks. There are no further things I can teach you right now.', 4)

addTaskStatusKeyword('addon', 'The time has come, my child. I sense great spiritual wisdom in you and I shall grant you a sign of your progress if you can fulfil my task.', 1)
addTaskStatusKeyword('addon', 'I shall grant you a sign of your progress as a shaman if you can fulfil my task.', 3)
addTaskStatusKeyword('addon', 'You have successfully passed all of my tasks. There are no further things I can teach you right now.', 4)
-- End Shaman Addons

-- Wooden Stake
keywordHandler:addKeyword({'stake'}, StdModule.say, {npcHandler = npcHandler, text = 'Ten prayers for a blessed stake? Don\'t tell me they made you travel whole Tibia for it! Listen, child, if you bring me a wooden stake, I\'ll bless it for you. <chuckles>'},
	function(player) return player:getStorageValue(Storage.FriendsAndTraders.TheBlessedStake) == 11 end,
	function(player) player:setStorageValue(Storage.FriendsAndTraders.TheBlessedStake, 12) player:addAchievement('Blessed!') player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE) end
)

local stakeKeyword = keywordHandler:addKeyword({'stake'}, StdModule.say, {npcHandler = npcHandler, text = 'Would you like to receive a spiritual prayer to bless your stake?'},
		function(player) return player:getStorageValue(Storage.FriendsAndTraders.TheBlessedStake) == 12 end
	)

	stakeKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = 'You don\'t have a wooden stake.', reset = true}, function(player) return player:getItemCount(5941) == 0 end)

	stakeKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = 'Sorry, but I\'m still exhausted from the last ritual. Please come back later.', reset = true},
		function(player) return player:getStorageValue(Storage.FriendsAndTraders.TheBlessedStakeWaitTime) >= os.time() end)

	stakeKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = '<mumblemumble> Sha Kesh Mar!', reset = true},
		function(player) return player:getItemCount(5941) > 0 end,
		function(player) player:setStorageValue(Storage.FriendsAndTraders.TheBlessedStakeWaitTime, os.time() + 7 * 86400) player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE) player:removeItem(5941, 1) player:addItem(5942, 1) end
	)
	stakeKeyword:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'Maybe another time.', reset = true})

-- Counterspell and Energy Field
local counterspellKeyword = keywordHandler:addKeyword({'counterspell'}, function(cid, type, msg, matches, node)
	local player = Player(cid)
	if not player then
		return false
	end

	-- Block if haven't delivered spellbook
	if player:getStorageValue(Storage.TheShatteredIsles.DragahsSpellbook) < 1 then
		npcHandler:say('You should not talk about things you don\'t know anything about.', cid)
		return true
	end

	-- Can start counterspell ritual
	if player:getStorageValue(Storage.TheShatteredIsles.TheCounterspell) == 0 then
		npcHandler:say('You mean, you are interested in a counterspell to cross the energy barrier on Goroma?', cid)
		npcHandler.topic[cid] = 11
		return true
	end

	-- Already started counterspell
	if player:getStorageValue(Storage.TheShatteredIsles.TheCounterspell) >= 1 and player:getStorageValue(Storage.TheShatteredIsles.TheCounterspell) < 4 then
		npcHandler:say('Did you bring the ' .. (player:getStorageValue(Storage.TheShatteredIsles.TheCounterspell) == 1 and 'fresh dead chicken' or player:getStorageValue(Storage.TheShatteredIsles.TheCounterspell) == 2 and 'fresh dead rat' or 'fresh dead black sheep') .. '?', cid)
		npcHandler.topic[cid] = 11 + player:getStorageValue(Storage.TheShatteredIsles.TheCounterspell)
		return true
	end

	-- Completed counterspell
	npcHandler:say('Hm. I don\'t think you need another one of my counterspells to cross the barrier on Goroma.', cid)
	return true
end, {npcHandler = npcHandler})

keywordHandler:addAliasKeyword({'energy field'})


npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 5669, buy = 0, sell = 4000, subType = 0, name = "Mysterious Voodoo Skull"},
    {id = 5670, buy = 0, sell = 4000, subType = 0, name = "Enigmatic Voodoo Skull"},
    {id = 9969, buy = 0, sell = 4000, subType = 0, name = "black skull"},
    {id = 2798, buy = 0, sell = 500, subType = 0, name = "blood herb"},
    {id = 9447, buy = 0, sell = 10000, subType = 0, name = "blood goblet"},
}

-- Helper function to find shop item by id and subType (for fluid containers)
local function getShopItem(itemId, subType, isBuying)
    local itemType = ItemType(itemId)
    if itemType:isFluidContainer() then
        for _, item in ipairs(shopItems) do
            if item.id == itemId and item.subType == subType then
                return item
            end
        end
    end
    -- For non-fluid items, find the entry that matches the operation
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            if isBuying and item.buy > 0 then
                return item
            elseif not isBuying and item.sell > 0 then
                return item
            end
        end
    end
    -- Fallback to first match
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            return item
        end
    end
    return nil
end

local function openTradeWindow(cid, message, keywords, parameters, node)
    if not npcHandler:isFocused(cid) then return false end
    local player = Player(cid)
    if not player then return false end
    local npc = Npc(getNpcCid())
    local shopList = {}
    for _, item in ipairs(shopItems) do
        table.insert(shopList, {id = item.id, buy = item.buy, sell = item.sell, subType = item.subType or 0, name = item.name})
    end
    npc:openShopWindow(player, shopList, function() return true end, function() return true end)
    npcHandler:say('Take all the time you need to browse my wares.', cid)
    return true
end
keywordHandler:addKeyword({'trade'}, openTradeWindow, {npcHandler = npcHandler})


-- NpcType callbacks (MUST call setCurrentNpc first!)
npcType:eventType(NPCS_EVENT_APPEAR)
npcType:onAppear(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onCreatureAppear(creature)
end)

npcType:eventType(NPCS_EVENT_DISAPPEAR)
npcType:onDisappear(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onCreatureDisappear(creature)
end)

npcType:eventType(NPCS_EVENT_SAY)
npcType:onSay(function(npc, creature, type, message)
    setCurrentNpc(npc)
    npcHandler:onCreatureSay(creature, type, message)
end)

npcType:eventType(NPCS_EVENT_THINK)
npcType:onThink(function(npc, interval)
    setCurrentNpc(npc)
    npcHandler:onThink()
end)

npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
npcType:onCloseChannel(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onPlayerCloseChannel(creature)
end)

npcType:eventType(NPCS_EVENT_SELLITEM)
npcType:onSellItem(function(npc, player, itemId, subType, amount, ignoreEquipped)
    local shopItem = getShopItem(itemId, subType, false)
    if not shopItem or shopItem.sell <= 0 then return false end
    local totalPrice = amount * shopItem.sell
    local itemName = shopItem.name or ItemType(itemId):getName()
    
    local itemSubType = -1
    if ItemType(itemId):isFluidContainer() then
        itemSubType = subType
    end
    
    if doPlayerSellItem(player:getId(), itemId, amount, totalPrice, itemSubType, ignoreEquipped) then
        player:sendTextMessage(MESSAGE_INFO_DESCR, "Sold " .. amount .. "x " .. shopItem.name .. " for " .. (amount * shopItem.sell) .. " gold.")
        return true
    end
    player:sendCancelMessage("You do not have this object.")
    return false
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
