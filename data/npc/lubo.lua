-- Lubo - Converted from XML to Lua NpcType
-- Original XML: data/npc/Lubo.xml
-- Original Script: data/npc/scripts/Lubo.lua

local npcName = "Lubo"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a lubo")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 115, lookBody = 39, lookLegs = 96, lookFeet = 118, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Stop by and rest a while, tired adventurer! Have a look at my wares!'} }
npcHandler:addModule(VoiceModule:new(voices))

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	local addonProgress = player:getStorageValue(Storage.OutfitQuest.Citizen.AddonBackpack)
	if msgcontains(msg, 'addon') or msgcontains(msg, 'outfit')
			or (addonProgress == 1 and msgcontains(msg, 'leather'))
			or ((addonProgress == 1 or addonProgress == 2) and msgcontains(msg, 'backpack')) then
		if addonProgress < 1 then
			npcHandler:say('Sorry, the backpack I wear is not for sale. It\'s handmade from rare minotaur leather.', cid)
			npcHandler.topic[cid] = 1
		elseif addonProgress == 1 then
			npcHandler:say('Ah, right, almost forgot about the backpack! Have you brought me 100 pieces of minotaur leather as requested?', cid)
			npcHandler.topic[cid] = 3
		elseif addonProgress == 2 then
			if player:getStorageValue(Storage.OutfitQuest.Citizen.AddonBackpackTimer) < os.time() then
				npcHandler:say('Just in time! Your backpack is finished. Here you go, I hope you like it.', cid)
				player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
				player:setStorageValue(Storage.OutfitQuest.Ref, math.min(0, player:getStorageValue(Storage.OutfitQuest.Ref) - 1))
				player:setStorageValue(Storage.OutfitQuest.Citizen.MissionBackpack, 0)
				player:setStorageValue(Storage.OutfitQuest.Citizen.AddonBackpack, 3)

				player:addOutfitAddon(136, 1)
				player:addOutfitAddon(128, 1)
			else
				npcHandler:say('Uh... I didn\'t expect you to return that early. Sorry, but I\'m not finished yet with your backpack. I\'m doing the best I can, promised.', cid)
			end
		elseif addonProgress == 3 then
			npcHandler:say('Sorry, but I can only make one backpack per person, else I\'d have to close my shop and open a leather manufactory.', cid)
		end
		return true
	end

	if npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'backpack') or msgcontains(msg, 'minotaur') or msgcontains(msg, 'leather') then
			npcHandler:say('Well, if you really like this backpack, I could make one for you, but minotaur leather is hard to come by these days. Are you willing to put some work into this?', cid)
			npcHandler.topic[cid] = 2
		end

	elseif npcHandler.topic[cid] == 2 then
		if msgcontains(msg, 'yes') then
			player:setStorageValue(Storage.OutfitQuest.DefaultStart, 1)
			player:setStorageValue(Storage.OutfitQuest.Ref, math.max(0, player:getStorageValue(Storage.OutfitQuest.Ref)) + 1)
			player:setStorageValue(Storage.OutfitQuest.Citizen.AddonBackpack, 1)
			player:setStorageValue(Storage.OutfitQuest.Citizen.MissionBackpack, 1)
			npcHandler:say('Alright then, if you bring me 100 pieces of fine minotaur leather I will see what I can do for you. You probably have to kill really many minotaurs though... so good luck!', cid)
			npcHandler:releaseFocus(cid)
		else
			npcHandler:say('Sorry, but I don\'t run a welfare office, you know... no pain, no gain.', cid)
		end
		npcHandler.topic[cid] = 0

	elseif npcHandler.topic[cid] == 3 then
		if msgcontains(msg, 'yes') then
			if player:getItemCount(5878) < 100 then
				npcHandler:say('Sorry, but that\'s not enough leather yet to make one of these backpacks. Would you rather like to buy a normal backpack for 10 gold?', cid)
			else
				npcHandler:say('Great! Alright, I need a while to finish this backpack for you. Come ask me later, okay?', cid)

				player:removeItem(5878, 100)

				player:setStorageValue(Storage.OutfitQuest.Citizen.MissionBackpack, 2)
				player:setStorageValue(Storage.OutfitQuest.Citizen.AddonBackpack, 2)
				player:setStorageValue(Storage.OutfitQuest.Citizen.AddonBackpackTimer, os.time() + 2 * 60 * 60)
			end
		else
			npcHandler:say('I know, it\'s quite some work... don\'t lose heart, just keep killing minotaurs and you\'ll eventually get lucky. Would you rather like to buy a normal backpack for 10 gold?', cid)
		end
		npcHandler.topic[cid] = 0
	end
end

keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I am selling equipment for adventurers. If you need anything, let me know.'})
keywordHandler:addKeyword({'dog'}, StdModule.say, {npcHandler = npcHandler, text = 'This is Ruffy my dog, please don\'t do him any harm.'})
keywordHandler:addKeyword({'offer'}, StdModule.say, {npcHandler = npcHandler, text = 'I sell torches, fishing rods, worms, ropes, water hoses, backpacks, apples, and maps.'})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = 'I am Lubo, the owner of this shop.'})
keywordHandler:addKeyword({'maps'}, StdModule.say, {npcHandler = npcHandler, text = 'Oh! I\'m sorry, I sold the last one just five minutes ago.'})
keywordHandler:addKeyword({'hat'}, StdModule.say, {npcHandler = npcHandler, text = 'My hat? Hanna made this one for me.'})
keywordHandler:addKeyword({'finger'}, StdModule.say, {npcHandler = npcHandler, text = 'Oh, you sure mean this old story about the mage Dago, who lost two fingers when he conjured a dragon.'})
keywordHandler:addKeyword({'pet'}, StdModule.say, {npcHandler = npcHandler, text = 'There are some strange stories about a magicians pet names. Ask Hoggle about it.'})

npcHandler:setMessage(MESSAGE_GREET, 'Welcome to my adventurer shop, |PLAYERNAME|! What do you need? Ask me for a {trade} to look at my wares.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Good bye, |PLAYERNAME|.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Good bye.')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2578, buy = 0, sell = 75, subType = 0, name = "closed trap"},
    {id = 2416, buy = 0, sell = 50, subType = 0, name = "crowbar"},
    {id = 2580, buy = 0, sell = 40, subType = 0, name = "fishing rod"},
    {id = 2420, buy = 0, sell = 6, subType = 0, name = "machete"},
    {id = 2553, buy = 0, sell = 15, subType = 0, name = "pick"},
    {id = 2120, buy = 0, sell = 15, subType = 0, name = "rope"},
    {id = 2550, buy = 0, sell = 10, subType = 0, name = "scythe"},
    {id = 2554, buy = 0, sell = 8, subType = 0, name = "shovel"},
    {id = 2036, buy = 0, sell = 6, subType = 0, name = "watch"},
    {id = 2556, buy = 0, sell = 15, subType = 0, name = "wooden hammer"},
    {id = 1988, buy = 25, sell = 0, subType = 0, name = "backpack"},
    {id = 1989, buy = 6, sell = 0, subType = 0, name = "basket"},
    {id = 2007, buy = 3, sell = 0, subType = 0, name = "bottle"},
    {id = 2005, buy = 4, sell = 0, subType = 0, name = "bucket"},
    {id = 2057, buy = 8, sell = 0, subType = 0, name = "candelabrum"},
    {id = 2047, buy = 2, sell = 0, subType = 0, name = "candlestick"},
    {id = 2578, buy = 280, sell = 0, subType = 0, name = "closed trap"},
    {id = 2416, buy = 260, sell = 0, subType = 0, name = "crowbar"},
    {id = 2580, buy = 150, sell = 0, subType = 0, name = "fishing rod"},
    {id = 2420, buy = 35, sell = 0, subType = 0, name = "machete"},
    {id = 2553, buy = 50, sell = 0, subType = 0, name = "pick"},
    {id = 1990, buy = 10, sell = 0, subType = 0, name = "present"},
    {id = 2674, buy = 3, sell = 0, subType = 0, name = "red apple"},
    {id = 2120, buy = 50, sell = 0, subType = 0, name = "rope"},
    {id = 2550, buy = 50, sell = 0, subType = 0, name = "scythe"},
    {id = 2554, buy = 50, sell = 0, subType = 0, name = "shovel"},
    {id = 2036, buy = 20, sell = 0, subType = 0, name = "watch"},
    {id = 2031, buy = 10, sell = 0, subType = 1, name = "waterskin"},
    {id = 3976, buy = 1, sell = 0, subType = 0, name = "worm"},
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
