-- Arito - Converted from XML to Lua NpcType
-- Original XML: data/npc/Arito.xml
-- Original Script: data/npc/scripts/Arito.lua

local npcName = "Arito"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a arito")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 132, lookHead = 59, lookBody = 111, lookLegs = 99, lookFeet = 115})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)

-- Nomad Parchment item ID
local NOMAD_PARCHMENT = 8267

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	local questState = player:getStorageValue(Storage.TibiaTales.AritosTask)

	-- Player mentions "nomads" with the deathlist parchment
	if msgcontains(msg, "nomads") then
		-- Quest not started - check if player has the Nomad Parchment
		if questState <= 0 then
			if player:getItemCount(NOMAD_PARCHMENT) >= 1 then
				npcHandler:say({
					'What?? My name on a deathlist which you retrieved from a nomad?? Show me!! ...',
					'Oh my god! They found me! You must help me! Please !!!! Are you willing to do that?'
				}, cid)
				npcHandler.topic[cid] = 1
			else
				npcHandler:say('Nomads? I don\'t know what you are talking about.', cid)
			end
		-- Quest in progress - player talked to Muhad
		elseif questState == 2 then
			npcHandler:say('Thank god you are back!! Did you find....err...what we were talking about??', cid)
			npcHandler.topic[cid] = 3
		-- Quest already completed
		elseif questState >= 3 then
			npcHandler:say('I am forever grateful for your help with the nomads. I can finally live in peace.', cid)
		else
			npcHandler:say('Please hurry! Go to the nomad cave and speak with their leader!', cid)
		end
		return true
	end

	-- Player says "yes" to help
	if msgcontains(msg, "yes") then
		-- Agreeing to help Arito
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				'Thank you thousand times! Well, I think I start telling you what I think they are after... ',
				'You have to know, I was one of them before I opened that shop here. Sure they fear about their hideout being revealed by me. Please go to the north, there is a small cave in the mountains with a rock in the middle. ...',
				'If you stand in front of it, place a scimitar - which is the weapon of the nomads - left of you and make a sacrifice to the earth by pouring some water on the floor to your right. ...',
				'The entrance to their hideout will be revealed in front of you. I don\'t know who is in charge there right now but please tell him that I won\'t spoil their secret... ',
				'... well, I just told you but anyway .... I won\'t tell it to anybody else. Now hurry up before they get here !!'
			}, cid)
			if player:getStorageValue(Storage.TibiaTales.DefaultStart) <= 0 then
				player:setStorageValue(Storage.TibiaTales.DefaultStart, 1)
			end
			player:setStorageValue(Storage.TibiaTales.AritosTask, 1)
			npcHandler.topic[cid] = 0
			return true
		-- Confirming return from Muhad
		elseif npcHandler.topic[cid] == 3 then
			npcHandler:say('And what did they say?? Do I have to give up everything here? Come on tell me!!', cid)
			npcHandler.topic[cid] = 4
			return true
		end
	end

	-- Player says "acquitted" to complete the quest
	if msgcontains(msg, "acquitted") then
		if npcHandler.topic[cid] == 4 or questState == 2 then
			npcHandler:say('These are great news!! Thank you for your help! I don\'t have much, but without you I wouldn\'t have anything so please take this as a reward.', cid)
			player:setStorageValue(Storage.TibiaTales.AritosTask, 3)
			player:addItem(2152, 50) -- 50 platinum coins
			npcHandler.topic[cid] = 0
			return true
		end
	end

	-- Player says "no" - decline to help
	if msgcontains(msg, "no") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say('Please reconsider! My life is in danger!', cid)
			npcHandler.topic[cid] = 0
			return true
		end
	end

	return true
end

local voices = { {text = 'Come in, have a drink and something to eat.'} }
npcHandler:addModule(VoiceModule:new(voices))

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, "Be mourned, pilgrim in flesh. Be mourned in my tavern.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Do visit us again.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Do visit us again.")
npcHandler:setMessage(MESSAGE_SENDTRADE, "Sure, browse through my offers.")


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2689, buy = 8, sell = 0, subType = 0, name = "bread"},
    {id = 2696, buy = 12, sell = 0, subType = 0, name = "cheese"},
    {id = 2667, buy = 6, sell = 0, subType = 0, name = "fish"},
    {id = 2671, buy = 16, sell = 0, subType = 0, name = "ham"},
    {id = 2666, buy = 10, sell = 0, subType = 0, name = "meat"},
    {id = 2012, buy = 2, sell = 0, subType = 5, name = "mug of lemonade"},
    {id = 2006, buy = 10, sell = 0, subType = 15, name = "wine"},
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
