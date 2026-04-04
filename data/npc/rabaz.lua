-- Rabaz - Converted from XML to Lua NpcType
-- Original XML: data/npc/Rabaz.xml
-- Original Script: data/npc/scripts/Rabaz.lua

local npcName = "Rabaz"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a rabaz")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookHead = 39, lookBody = 38, lookLegs = 1, lookFeet = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.TibiaTales.AnInterestInBotany) < 1 then
			npcHandler.topic[cid] = 1
				npcHandler:say({
					"Why yes, there is indeed some minor issue I could need your help with. I was always a friend of nature and it was not recently I discovered the joys of plants, growths, of all the flora around us. ...",
					"Botany my friend. The study of plants is of great importance for our future. Many of the potions we often depend on are made of plants you know. Plants can help us tending our wounds, cure us from illness or injury. ...",
					"I am currently writing an excessive compilation of all the knowledge I have gathered during my time here in Farmine and soon hope to publish it as 'Rabaz' Unabridged Almanach Of Botany'. ...",
					"However, to actually complete my botanical epitome concerning Zao, I would need someone to enter these dangerous lands. Someone able to get closer to the specimens than I can. ...",
					"And this is where you come in. There are two extremely rare species I need samples from. Typically not easy to come by but it should not be necessary to venture too far into Zao to find them. ...",
					"Explore the anterior outskirts of Zao, use my almanach and find the two specimens with missing samples on their pages. The almanach can be found in a chest in my storage, next to my shop. It's the door over there. ...",
					"If you lose it I will have to write a new one and put it in there again - which will undoubtedly take me a while. So keep an eye on it on your travels. ...",
					"Once you find what I need, best use a knife to carefully cut and gather a leaf or a scrap of their integument and press it directly under their appropriate entry into my botanical almanach. ...",
					"Simply return to me after you have done that and we will discuss your reward. What do you say, are you in?"
				}, cid)
		elseif player:getStorageValue(Storage.TibiaTales.AnInterestInBotany) == 3 then
			npcHandler.topic[cid] = 2
			npcHandler:say("Well fantastic work, you gathered both samples! Now I can continue my work on the almanach, thank you very much for your help indeed. Can I take a look at my book please?", cid)
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			player:setStorageValue(Storage.TibiaTales.DefaultStart, 1)
			player:setStorageValue(Storage.TibiaTales.AnInterestInBotany, 1)
			player:setStorageValue(Storage.TibiaTales.AnInterestInBotanyChest, 0)
			npcHandler:say("Yes? Yes! That's the enthusiasm I need! Remember to bring a sharp knife to gather the samples, plants - even mutated deformed plants - are very sensitive you know. Off you go and be careful out there, Zao is no place for the feint hearted mind you.", cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 2 then
			if player:removeItem(12655, 1) then
				player:addItem(12656, 1)
				player:addItem(2152, 10)
				player:addExperience(3000, true)
				player:setStorageValue(Storage.TibiaTales.AnInterestInBotany, 4)
				npcHandler:say({
					"Ah, thank you. Now look at that texture and fine colour, simply marvellous. ...",
					"I hope the sun in the steppe did not exhaust you too much? Shellshock. A dangerous foe in the world of field science and exploration. ...",
					"Here, I always wore this comfortable hat when travelling, take it. It may be of use for you on further reconnaissances in Zao. Again you have my thanks, friend."
				}, cid)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("Oh, you don't have my book.", cid)
				npcHandler.topic[cid] = 0
			end
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 7634, buy = 0, sell = 5, subType = 0, name = "empty potion flask"},
    {id = 7635, buy = 0, sell = 5, subType = 0, name = "empty potion flask"},
    {id = 7636, buy = 0, sell = 5, subType = 0, name = "empty potion flask"},
    {id = 7735, buy = 0, sell = 299, subType = 0, name = "spellwand"},
    {id = 2151, buy = 0, sell = 320, subType = 0, name = "talon"},
    {id = 2006, buy = 0, sell = 5, subType = 0, name = "vial"},
    {id = 2274, buy = 45, sell = 0, subType = 0, name = "avalanche rune"},
    {id = 2260, buy = 10, sell = 0, subType = 0, name = "blank rune"},
    {id = 2261, buy = 15, sell = 0, subType = 0, name = "destroy field rune"},
    {id = 2277, buy = 38, sell = 0, subType = 0, name = "energy field rune"},
    {id = 2279, buy = 85, sell = 0, subType = 0, name = "energy wall rune"},
    {id = 2313, buy = 31, sell = 0, subType = 0, name = "explosion rune"},
    {id = 2305, buy = 117, sell = 0, subType = 0, name = "fire bomb rune"},
    {id = 2301, buy = 28, sell = 0, subType = 0, name = "fire field rune"},
    {id = 2303, buy = 61, sell = 0, subType = 0, name = "fire wall rune"},
    {id = 2304, buy = 45, sell = 0, subType = 0, name = "great fireball rune"},
    {id = 7591, buy = 190, sell = 0, subType = 0, name = "great health potion"},
    {id = 7590, buy = 120, sell = 0, subType = 0, name = "great mana potion"},
    {id = 8472, buy = 190, sell = 0, subType = 0, name = "great spirit potion"},
    {id = 2183, buy = 15000, sell = 0, subType = 0, name = "hailstorm rod"},
    {id = 7618, buy = 34, sell = 0, subType = 0, name = "health potion"},
    {id = 2311, buy = 12, sell = 0, subType = 0, name = "heavy magic missile rune"},
    {id = 2265, buy = 95, sell = 0, subType = 0, name = "intense healing rune"},
    {id = 2287, buy = 4, sell = 0, subType = 0, name = "light magic missile rune"},
    {id = 7620, buy = 50, sell = 0, subType = 0, name = "mana potion"},
    {id = 2186, buy = 1000, sell = 0, subType = 0, name = "moonlight rod"},
    {id = 2185, buy = 5000, sell = 0, subType = 0, name = "necrotic rod"},
    {id = 2285, buy = 21, sell = 0, subType = 0, name = "poison field rune"},
    {id = 2289, buy = 52, sell = 0, subType = 0, name = "poison wall rune"},
    {id = 2182, buy = 500, sell = 0, subType = 0, name = "snakebite rod"},
    {id = 2175, buy = 150, sell = 0, subType = 0, name = "spellbook"},
    {id = 8912, buy = 22000, sell = 0, subType = 0, name = "springsprout rod"},
    {id = 2292, buy = 12, sell = 0, subType = 0, name = "stalagmite rune"},
    {id = 7588, buy = 75, sell = 0, subType = 0, name = "strong health potion"},
    {id = 7589, buy = 80, sell = 0, subType = 0, name = "strong mana potion"},
    {id = 2268, buy = 108, sell = 0, subType = 0, name = "sudden death rune"},
    {id = 2181, buy = 10000, sell = 0, subType = 0, name = "terra rod"},
    {id = 2273, buy = 175, sell = 0, subType = 0, name = "ultimate healing rune"},
    {id = 8473, buy = 310, sell = 0, subType = 0, name = "ultimate health potion"},
    {id = 8910, buy = 22000, sell = 0, subType = 0, name = "Underworld Rod"},
    {id = 2189, buy = 10000, sell = 0, subType = 0, name = "wand of cosmic energy"},
    {id = 2188, buy = 5000, sell = 0, subType = 0, name = "wand of decay"},
    {id = 2191, buy = 1000, sell = 0, subType = 0, name = "wand of dragonbreath"},
    {id = 2187, buy = 15000, sell = 0, subType = 0, name = "wand of inferno"},
    {id = 8920, buy = 18000, sell = 0, subType = 0, name = "wand of starstorm"},
    {id = 8921, buy = 7500, sell = 0, subType = 0, name = "wand of draconia"},
    {id = 2190, buy = 500, sell = 0, subType = 0, name = "wand of vortex"},
    {id = 8922, buy = 22000, sell = 0, subType = 0, name = "wand of voodoo"},
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
