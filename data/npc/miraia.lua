-- Miraia - Converted from XML to Lua NpcType
-- Original XML: data/npc/Miraia.xml
-- Original Script: data/npc/scripts/Miraia.lua

local npcName = "Miraia"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a miraia")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 150, lookHead = 114, lookLegs = 7, lookFeet = 132, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local message = {}

local config = {
	['ape fur'] = {
		itemId = 5883,
		count = 100,
		storageValue = 1,
		text = {
			'Have you really managed to fulfil the task and brought me 100 pieces of ape fur?',
			'Only ape fur is good enough to touch the feet of our Caliph.',
			'Ahhh, this softness! I\'m impressed, |PLAYERNAME|. You\'re on the best way to earn that turban. Now, please retrieve 100 fish fins.'
		}
	},
	['fish fins'] = {
		itemId = 5895,
		count = 100,
		storageValue = 2,
		text = {
			'Were you able to discover the undersea race and retrieved 100 fish fins?',
			'I really wonder what the explorer society is up to. Actually I have no idea how they managed to dive unterwater.',
			'I never thought you\'d make it, |PLAYERNAME|. Now we only need two enchanted chicken wings to start our waterwalking test!'
		}
	},
	['enchanted chicken wings'] = {
		itemId = 5891,
		count = 2,
		storageValue = 3,
		text = {
			'Were you able to get hold of two enchanted chicken wings?',
			'Enchanted chicken wings are actually used to make boots of haste, so they could be magically extracted again. Djinns are said to be good at that.',
			'Great, thank you very much. Just bring me 100 pieces of blue cloth now and I will happily show you how to make a turban.'
		}
	},
	['blue cloth'] = {
		itemId = 5912,
		count = 100,
		storageValue = 4,
		text = {
			'Ah, have you brought the 100 pieces of blue cloth?',
			'It\'s a great material for turbans.',
			'Ah! Congratulations - I hope this veil will turn out as beautiful as you are. Here, I\'ll do it for you.'
		}
	}
}

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, 'outfit') then
		npcHandler:say(player:getSex() == PLAYERSEX_FEMALE and 'Hehe, would you like to wear a pretty veil like I do? Well... I could help you, but you would have to complete a task first.' or 'My veil? No, I will definitely not lift it for you! If you are looking for an addon, go talk to Razan.', cid)
	elseif msgcontains(msg, 'task') then
		if player:getSex() == PLAYERSEX_MALE then
			npcHandler:say('Uh... I don\'t think that I have work for you right now. If you need a job, go talk to Razan.', cid)
			return true
		end
		if player:getStorageValue(Storage.OutfitQuest.secondOrientalAddon) < 1 then
			npcHandler:say('You mean, you would like to prove that you deserve to wear such a veil?', cid)
			npcHandler.topic[cid] = 1
		end
	elseif config[msg] and npcHandler.topic[cid] == 0 then
		if player:getStorageValue(Storage.OutfitQuest.secondOrientalAddon) == config[msg].storageValue then
			npcHandler:say(config[msg].text[1], cid)
			npcHandler.topic[cid] = 3
			message[cid] = msg
		else
			npcHandler:say(config[msg].text[2], cid)
		end
	elseif msgcontains(msg, 'scarab cheese') then
		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission03) == 1 then
			npcHandler:say('Let me cover my nose before I get this for you... Would you REALLY like to buy scarab cheese for 100 gold?', cid)
		elseif player:getStorageValue(Storage.TheTravellingTraderQuest.Mission03) == 2 then
			npcHandler:say('Oh the last cheese molded? Would you like to buy another one for 100 gold?', cid)
		end
		npcHandler.topic[cid] = 4
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				'Alright, then listen to the following requirements. We are currently in dire need of ape fur since the Caliph has requested a new bathroom carpet. ...',
				'Thus, please bring me 100 pieces of ape fur. Secondly, it came to our ears that the explorer society has discovered a new undersea race of fishmen. ...',
				'Their fins are said to allow humans to walk on water! Please bring us 100 of these fish fin. ...',
				'Third, if the plan of walking on water should fail, we need enchanted chicken wings to prevent the testers from drowning. Please bring me two. ...',
				'Last but not least, just drop by with 100 pieces of blue cloth and I will happily show you how to make a turban. ...',
				'Did you understand everything I told you and are willing to handle this task?'
			}, cid)
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 2 then
			if player:getStorageValue(Storage.OutfitQuest.DefaultStart) ~= 1 then
				player:setStorageValue(Storage.OutfitQuest.DefaultStart, 1)
			end
			player:setStorageValue(Storage.OutfitQuest.secondOrientalAddon, 1)
			npcHandler:say('Excellent! Come back to me once you have collected 100 pieces of ape fur.', cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 then
			local targetMessage = config[message[cid]]
			if not player:removeItem(targetMessage.itemId, targetMessage.count) then
				npcHandler:say('That is a shameless lie.', cid)
				npcHandler.topic[cid] = 0
				return true
			end
			player:setStorageValue(Storage.OutfitQuest.secondOrientalAddon, player:getStorageValue(Storage.OutfitQuest.secondOrientalAddon) + 1)
			if player:getStorageValue(Storage.OutfitQuest.secondOrientalAddon) == 5 then
				player:addOutfitAddon(146, 2)
				player:addOutfitAddon(150, 2)
				player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
			end
			npcHandler:say(targetMessage.text[3], cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 4 then
			if player:getMoney() + player:getBankBalance() >= 100 then
				player:setStorageValue(Storage.TheTravellingTraderQuest.Mission03, 2)
				player:addItem(8112, 1)
				player:removeMoneyNpc(100)
				npcHandler:say('Here it is.', cid)
			else
				npcHandler:say('You don\'t have enough money.', cid)
			end
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'no') and npcHandler.topic[cid] ~= 0 then
		npcHandler:say('What a pity.', cid)
		npcHandler.topic[cid] = 0
	end
	return true
end

local function onReleaseFocus(cid)
	message[cid] = nil
end

keywordHandler:addKeyword({'drink'}, StdModule.say, {npcHandler = npcHandler, text = 'I can offer you lemonade, camel milk, and water. If you\'d like to see my offers, ask me for a {trade}.'})
keywordHandler:addKeyword({'food'}, StdModule.say, {npcHandler = npcHandler, text = 'Are you looking for food? I have bread, cheese, ham, and meat. If you\'d like to see my offers, ask me for a {trade}.'})

npcHandler:setMessage(MESSAGE_GREET, 'Daraman\'s blessings, |PLAYERNAME|. Welcome to the Enlightened Oasis. Sit down, have a {drink} or some {food}!')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Daraman\'s blessings. Come back soon.')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setCallback(CALLBACK_ONRELEASEFOCUS, onReleaseFocus)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2666, buy = 5, sell = 0, subType = 0, name = "meat"},
    {id = 2671, buy = 8, sell = 0, subType = 0, name = "ham"},
    {id = 2689, buy = 4, sell = 0, subType = 0, name = "bread"},
    {id = 2696, buy = 6, sell = 0, subType = 0, name = "cheese"},
    {id = 2012, buy = 3, sell = 0, subType = 5, name = "mug of lemonade"},
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

npcType:eventType(NPCS_EVENT_BUYITEM)
npcType:onBuyItem(function(npc, player, itemId, subType, amount, ignoreCap, inBackpacks)
    local shopItem = getShopItem(itemId, subType, true)
    if not shopItem or shopItem.buy <= 0 then return false end
    local totalCost = amount * shopItem.buy
    if player:getTotalMoney() < totalCost then
        player:sendCancelMessage("You don't have enough money.")
        return false
    end
    local itemSubType = shopItem.subType or 1
    local bought = doNpcSellItem(player:getId(), itemId, amount, itemSubType, ignoreCap, inBackpacks, ITEM_BACKPACK)
    if bought == 0 then
        player:sendCancelMessage("You do not have enough capacity.")
        return false
    end
    player:removeTotalMoney(bought * shopItem.buy)
    player:sendTextMessage(MESSAGE_INFO_DESCR, "Bought " .. bought .. "x " .. shopItem.name .. " for " .. (bought * shopItem.buy) .. " gold.")
    return true
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
