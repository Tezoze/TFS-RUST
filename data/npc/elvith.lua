-- Elvith - Converted from XML to Lua NpcType
-- Original XML: data/npc/Elvith.xml
-- Original Script: data/npc/scripts/Elvith.lua

local npcName = "Elvith"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a elvith")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 159, lookHead = 76, lookBody = 3, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I sell musical instruments of many kinds.'})
keywordHandler:addKeyword({'instruments'}, StdModule.say, {npcHandler = npcHandler, text = 'I sell lyres, lutes, drums, and simple fanfares.'})
keywordHandler:addKeyword({'music'}, StdModule.say, {npcHandler = npcHandler, text = 'Music is an attempt to condensate emotions in harmonies and save them for the times to come.'})
keywordHandler:addKeyword({'time'}, StdModule.say, {npcHandler = npcHandler, text = 'Time has its own song. Close your eyes and listen to the symphony of the seasons.'})
keywordHandler:addKeyword({'song'}, StdModule.say, {npcHandler = npcHandler, text = 'Everything is a song. Life, death, history ... everything. To listen to the song of something is the first step to understand it.'})
keywordHandler:addKeyword({'melody'}, StdModule.say, {npcHandler = npcHandler, text = 'Everything is a song. Life, death, history ... everything. To listen to the song of something is the first step to understand it.'})
keywordHandler:addKeyword({'elf'}, StdModule.say, {npcHandler = npcHandler, text = 'We are the most graceful of all races. We feel the music of the universe in our hearts and souls.'})
keywordHandler:addKeyword({'kuridai'}, StdModule.say, {npcHandler = npcHandler, text = 'They could dig some halls for a big musical event, but they won\'t listen to me about that matter.'})
keywordHandler:addKeyword({'teshial'}, StdModule.say, {npcHandler = npcHandler, text = 'I bet they were great musicians.'})
keywordHandler:addKeyword({'crunor'}, StdModule.say, {npcHandler = npcHandler, text = 'That is some god the humans worship. Our pople are not interested in this gods anymore.'})
keywordHandler:addKeyword({'human'}, StdModule.say, {npcHandler = npcHandler, text = 'They are too loud and don\'t even understand the concept of a melody.'})
keywordHandler:addKeyword({'deraisim'}, StdModule.say, {npcHandler = npcHandler, text = 'The other deraisim are too much concerned with mastering the nature so they don\'t listen to its music anymore.'})
keywordHandler:addKeyword({'cenath'}, StdModule.say, {npcHandler = npcHandler, text = 'The Cenath think they know the \'art\' but the only true art is the music.'})
keywordHandler:addKeyword({'troll'}, StdModule.say, {npcHandler = npcHandler, text = 'I went down to the mines and tried to lighten up their spirit, the foolish creatures did not listen to my songs, though.'})
keywordHandler:addKeyword({'magic'}, StdModule.say, {npcHandler = npcHandler, text = 'Sorry, I don\'t feel like teaching magic today.'})
keywordHandler:addKeyword({'hellgate'}, StdModule.say, {npcHandler = npcHandler, text = 'For the worst of crimes, criminals are cast into hellgate. It is said no one can return from there. Since it is not actually forbidden to enter hellgate, you might convince Elathriel to grant you entrance.'})

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	if msgcontains(msg, 'songs of the forest') then
		npcHandler:say({
			'The last issue I had was bought by Randor Swiftfinger. He was banished through the hellgate and probably took the book with him ...',
			'I would not recommend seeking him or the book there, but of course it is possible.'
		}, cid)
	elseif msgcontains(msg, 'love poem') then
		npcHandler:say('Do you want to buy a poem scroll for 200 gold?', cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			npcHandler.topic[cid] = 0
			local player = Player(cid)
			if not player:removeMoneyNpc(200) then
				npcHandler:say('You don\'t have enough money.', cid)
				return true
			end

			player:addItem(8189, 1)
			npcHandler:say('Here it is.', cid)
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, 'Ashari |PLAYERNAME|.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Asha Thrazi.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Asha Thrazi.')

local focusModule = FocusModule:new()
focusModule:addGreetMessage({'hi', 'hello', 'ashari'})
focusModule:addFarewellMessage({'bye', 'farewell', 'asgha thrazi'})
npcHandler:addModule(focusModule)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2073, buy = 140, sell = 0, subType = 0, name = "drum"},
    {id = 2072, buy = 195, sell = 0, subType = 0, name = "lute"},
    {id = 2071, buy = 120, sell = 0, subType = 0, name = "lyre"},
    {id = 2075, buy = 150, sell = 0, subType = 0, name = "simple fanfare"},
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

npcType:register()
