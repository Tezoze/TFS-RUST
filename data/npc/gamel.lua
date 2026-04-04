-- Gamel - Converted from XML to Lua NpcType
-- Original XML: data/npc/Gamel.xml
-- Original Script: data/npc/scripts/Gamel.lua

local npcName = "Gamel"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a gamel")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 129, lookHead = 79, lookBody = 115, lookLegs = 115, lookFeet = 116})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Pssst!'} }
npcHandler:addModule(VoiceModule:new(voices))

local function greetCallback(cid)
	local player = Player(cid)

	if player:getStorageValue(Storage.secretService.AVINMission01) == 1 and player:getItemCount(14326) > 0 then
		player:setStorageValue(Storage.secretService.AVINMission01, 2)
		player:setStorageValue(12552, 2) -- Quest log: Gamel sent thugs
		npcHandler:say("I don't like the way you look. Help me boys!", cid)
		for i = 1, 2 do
			Game.createMonster("Bandit", Npc():getPosition())
		end
		npcHandler.topic[cid] = 0
	else
		npcHandler:setMessage(MESSAGE_GREET, "Pssst! Be silent. Do you wish to {buy} something?")
	end
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, "letter") then
		if player:getStorageValue(Storage.secretService.AVINMission01) == 2 then
			npcHandler:say("You have a letter for me?", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
		if player:removeItem(14326, 1) then
			player:setStorageValue(Storage.secretService.AVINMission01, 3)
			player:setStorageValue(12552, 3) -- Quest log: Gamel accepted letter
			npcHandler:say("Oh well. I guess I am still on the hook. Tell your 'uncle' I will proceed as he suggested.", cid)
			else
				npcHandler:say("You don't have any letter!", cid)
			end
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_WALKAWAY, "Bye. Tell others about... my little shop here.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Bye. Tell others about... my little shop here.")
npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2386, buy = 0, sell = 7, subType = 0, name = "axe"},
    {id = 2378, buy = 0, sell = 80, subType = 0, name = "battle axe"},
    {id = 2417, buy = 0, sell = 120, subType = 0, name = "battle hammer"},
    {id = 2449, buy = 0, sell = 5, subType = 0, name = "bone club"},
    {id = 2450, buy = 0, sell = 20, subType = 0, name = "bone sword"},
    {id = 2460, buy = 0, sell = 30, subType = 0, name = "brass helmet"},
    {id = 2395, buy = 0, sell = 118, subType = 0, name = "carlin sword"},
    {id = 2458, buy = 0, sell = 17, subType = 0, name = "chain helmet"},
    {id = 2382, buy = 0, sell = 1, subType = 0, name = "club"},
    {id = 2416, buy = 0, sell = 50, subType = 0, name = "crowbar"},
    {id = 2379, buy = 0, sell = 2, subType = 0, name = "dagger"},
    {id = 2387, buy = 0, sell = 260, subType = 0, name = "double axe"},
    {id = 2392, buy = 0, sell = 1000, subType = 0, name = "fire sword"},
    {id = 2381, buy = 0, sell = 400, subType = 0, name = "halberd"},
    {id = 2380, buy = 0, sell = 4, subType = 0, name = "hand axe"},
    {id = 2388, buy = 0, sell = 25, subType = 0, name = "hatchet"},
    {id = 2459, buy = 0, sell = 150, subType = 0, name = "iron helmet"},
    {id = 2412, buy = 0, sell = 35, subType = 0, name = "katana"},
    {id = 2461, buy = 0, sell = 4, subType = 0, name = "leather helmet"},
    {id = 2480, buy = 0, sell = 22, subType = 0, name = "legion helmet"},
    {id = 2397, buy = 0, sell = 51, subType = 0, name = "longsword"},
    {id = 2398, buy = 0, sell = 30, subType = 0, name = "mace"},
    {id = 2394, buy = 0, sell = 100, subType = 0, name = "morning star"},
    {id = 2428, buy = 0, sell = 350, subType = 0, name = "orcish axe"},
    {id = 2384, buy = 0, sell = 5, subType = 0, name = "rapier"},
    {id = 2385, buy = 0, sell = 12, subType = 0, name = "sabre"},
    {id = 2406, buy = 0, sell = 10, subType = 0, name = "short sword"},
    {id = 2405, buy = 0, sell = 3, subType = 0, name = "sickle"},
    {id = 2559, buy = 0, sell = 5, subType = 0, name = "small axe"},
    {id = 2481, buy = 0, sell = 16, subType = 0, name = "soldier helmet"},
    {id = 2383, buy = 0, sell = 240, subType = 0, name = "spike sword"},
    {id = 2457, buy = 0, sell = 293, subType = 0, name = "steel helmet"},
    {id = 2448, buy = 0, sell = 10, subType = 0, name = "studded club"},
    {id = 2482, buy = 0, sell = 20, subType = 0, name = "studded helmet"},
    {id = 2410, buy = 0, sell = 2, subType = 0, name = "throwing knife"},
    {id = 2377, buy = 0, sell = 450, subType = 0, name = "two handed sword"},
    {id = 2473, buy = 0, sell = 66, subType = 0, name = "viking helmet"},
    {id = 2391, buy = 0, sell = 470, subType = 0, name = "war hammer"},
    {id = 2386, buy = 20, sell = 0, subType = 0, name = "axe"},
    {id = 2378, buy = 235, sell = 0, subType = 0, name = "battle axe"},
    {id = 2417, buy = 350, sell = 0, subType = 0, name = "battle hammer"},
    {id = 2450, buy = 75, sell = 0, subType = 0, name = "bone sword"},
    {id = 2460, buy = 120, sell = 0, subType = 0, name = "brass helmet"},
    {id = 2395, buy = 473, sell = 0, subType = 0, name = "carlin sword"},
    {id = 2458, buy = 52, sell = 0, subType = 0, name = "chain helmet"},
    {id = 2382, buy = 5, sell = 0, subType = 0, name = "club"},
    {id = 2416, buy = 260, sell = 0, subType = 0, name = "crowbar"},
    {id = 2379, buy = 5, sell = 0, subType = 0, name = "dagger"},
    {id = 2380, buy = 8, sell = 0, subType = 0, name = "hand axe"},
    {id = 2459, buy = 390, sell = 0, subType = 0, name = "iron helmet"},
    {id = 2461, buy = 12, sell = 0, subType = 0, name = "leather helmet"},
    {id = 2397, buy = 160, sell = 0, subType = 0, name = "longsword"},
    {id = 2398, buy = 90, sell = 0, subType = 0, name = "mace"},
    {id = 2394, buy = 430, sell = 0, subType = 0, name = "morning star"},
    {id = 2384, buy = 15, sell = 0, subType = 0, name = "rapier"},
    {id = 2385, buy = 35, sell = 0, subType = 0, name = "sabre"},
    {id = 2406, buy = 26, sell = 0, subType = 0, name = "short sword"},
    {id = 2405, buy = 7, sell = 0, subType = 0, name = "sickle"},
    {id = 2481, buy = 110, sell = 0, subType = 0, name = "soldier helmet"},
    {id = 2383, buy = 8000, sell = 0, subType = 0, name = "spike sword"},
    {id = 2401, buy = 40, sell = 0, subType = 0, name = "staff"},
    {id = 2457, buy = 580, sell = 0, subType = 0, name = "steel helmet"},
    {id = 2482, buy = 63, sell = 0, subType = 0, name = "studded helmet"},
    {id = 2410, buy = 25, sell = 0, subType = 0, name = "throwing knife"},
    {id = 2377, buy = 950, sell = 0, subType = 0, name = "two handed sword"},
    {id = 2473, buy = 265, sell = 0, subType = 0, name = "viking helmet"},
    {id = 2391, buy = 10000, sell = 0, subType = 0, name = "war hammer"},
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
