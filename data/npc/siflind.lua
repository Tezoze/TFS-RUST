-- Siflind - Converted from XML to Lua NpcType
-- Original XML: data/npc/Siflind.xml
-- Original Script: data/npc/scripts/Silfind.lua

local npcName = "Siflind"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a siflind")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 158, lookHead = 76, lookBody = 81, lookLegs = 95, lookFeet = 114, lookAddons = 1})
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
		if player:getStorageValue(Storage.TheIceIslands.Questline) == 5 then
			npcHandler:say("I heard you have already helped our cause. Are you interested in another mission, even when it requires you to travel to a distant land?", cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 7 then
			npcHandler:say("Well done. The termites caused just the distraction that we needed. Are you ready for the next step of my plan?", cid)
			npcHandler.topic[cid] = 3
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 9 then
			npcHandler:say("You saved the lives of many innocent animals. Thank you very much. If you are looking for another mission, just ask me.", cid)
			player:setStorageValue(Storage.TheIceIslands.Questline, 10)
			npcHandler.topic[cid] = 0
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 10 then
			npcHandler:say("Our warriors need a more potent yet more secure berserker elixir to fight our enemies. To brew it, I need several ingredients. The first things needed are 5 bat wings. Bring them to me and Ill tell you the next ingredients we need.", cid)
			player:setStorageValue(Storage.TheIceIslands.Questline, 11)
			player:setStorageValue(Storage.TheIceIslands.Mission05, 1) -- Questlog The Ice Islands Quest, Nibelor 4: Berserk Brewery
			npcHandler.topic[cid] = 0
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 11 then
			npcHandler:say("Do you have the 5 bat wings I requested?", cid)
			npcHandler.topic[cid] = 5
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 12 then
			npcHandler:say("The second things needed are 4 bear paws. Bring them to me and Ill tell you the next ingredients we need.", cid)
			player:setStorageValue(Storage.TheIceIslands.Questline, 13)
			player:setStorageValue(Storage.TheIceIslands.Mission05, 2) -- Questlog The Ice Islands Quest, Nibelor 4: Berserk Brewery
			npcHandler.topic[cid] = 0
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 13 then
			npcHandler:say("Do you have the 4 bear paws I requested?", cid)
			npcHandler.topic[cid] = 6
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 14 then
			npcHandler:say("The next things needed are 3 bonelord eyes. Bring them to me and Ill tell you the next ingredients we need.", cid)
			player:setStorageValue(Storage.TheIceIslands.Questline, 15)
			player:setStorageValue(Storage.TheIceIslands.Mission05, 3) -- Questlog The Ice Islands Quest, Nibelor 4: Berserk Brewery
			npcHandler.topic[cid] = 0
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 15 then
			npcHandler:say("Do you have the 3 bonelord eyes I requested?", cid)
			npcHandler.topic[cid] = 7
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 16 then
			npcHandler:say("The next things needed are 2 fish fins. Bring them to me and Ill tell you the next ingredients we need.", cid)
			player:setStorageValue(Storage.TheIceIslands.Questline, 17)
			player:setStorageValue(Storage.TheIceIslands.Mission05, 4) -- Questlog The Ice Islands Quest, Nibelor 4: Berserk Brewery
			npcHandler.topic[cid] = 0
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 17 then
			npcHandler:say("Do you have the 2 fish fins I requested?", cid)
			npcHandler.topic[cid] = 8
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 18 then
			npcHandler:say("The last thing needed is a green dragon scale. Bring them to me and Ill tell you the next ingredients we need.", cid)
			player:setStorageValue(Storage.TheIceIslands.Questline, 19)
			player:setStorageValue(Storage.TheIceIslands.Mission05, 5) -- Questlog The Ice Islands Quest, Nibelor 4: Berserk Brewery
			npcHandler.topic[cid] = 0
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 19 then
			npcHandler:say("Do you have the green dragon scale I requested?", cid)
			npcHandler.topic[cid] = 9
		else
		npcHandler:say("I have now no mission for you.", cid)
		npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "jug") then
		npcHandler:say("Do you want to buy a jug for 1000 gold?", cid)
		npcHandler.topic[cid] = 2
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				"I am pleased to hear that. On the isle of Tyrsung foreign hunters have set up camp. They are hunting the animals there with no mercy. We will haveto find something that distracts them from hunting ...",
				"Take this jug here and travel to the jungle of Tiquanda. There you will find a race of wood eating ants called termites. Use the jug on one of their hills to catch some of them ...",
				"Then find someone in Svargrond that brings you to Tyrsung. There, release the termites on the bottom of a mast in the hull of the hunters' ship. If you are done, report to me about your mission."
			}, cid)
			player:setStorageValue(Storage.TheIceIslands.Questline, 6)
			player:setStorageValue(Storage.TheIceIslands.Mission03, 1) -- Questlog The Ice Islands Quest, Nibelor 2: Ecological Terrorism
			player:addItem(7243, 1)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 2 then
			if player:getMoney() + player:getBankBalance() >= 1000 then
				player:removeMoneyNpc(1000)
				npcHandler:say("Here you are.", cid)
				npcHandler.topic[cid] = 0
				player:addItem(7243, 1)
			end
		elseif npcHandler.topic[cid] == 3 then
			npcHandler:say("Good! Now listen. To protect the animals there, we have to harm the profit of the hunters. Therefor, I ask you to ruin their best source of earnings. Are you willing to do that?", cid)
			npcHandler.topic[cid] = 4
		elseif npcHandler.topic[cid] == 4 then
			npcHandler:say("So let's proceed. Take this vial of paint. Travel to Tyrsung again and ruin as many pelts of baby seals as possible before the paint runs dry or freezes. Then return here to report about your mission. ", cid)
			player:addItem(7253, 1)
			player:setStorageValue(Storage.TheIceIslands.Questline, 8)
			player:setStorageValue(Storage.TheIceIslands.Mission04, 1) -- Questlog The Ice Islands Quest, Nibelor 3: Artful Sabotage
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 5 then -- Wings
			if player:removeItem(5894, 5) then
				npcHandler:say("Thank you very much.", cid)
				player:setStorageValue(Storage.TheIceIslands.Questline, 12)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("Come back when you do.", cid)
			end
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 6 then -- Paws
			if player:removeItem(5896, 4) then
				npcHandler:say("Thank you very much.", cid)
				player:setStorageValue(Storage.TheIceIslands.Questline, 14)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("Come back when you do.", cid)
			end
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 7 then -- Eyes
			if player:removeItem(5898, 3) then
				npcHandler:say("Thank you very much.", cid)
				player:setStorageValue(Storage.TheIceIslands.Questline, 16)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("Come back when you do.", cid)
			end
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 8 then -- Fins
			if player:removeItem(5895, 2) then
				npcHandler:say("Thank you very much.", cid)
				player:setStorageValue(Storage.TheIceIslands.Questline, 18)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("Come back when you do.", cid)
			end
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 9 then -- Scale
			if player:removeItem(5920, 1) then
				npcHandler:say("Thank you very much. This will help us to defend Svargrond. But I heard young Nilsor is in dire need of help. Please contact him immediately.", cid)
				player:setStorageValue(Storage.TheIceIslands.Questline, 20)
				player:setStorageValue(Storage.TheIceIslands.Mission05, 6) -- Questlog The Ice Islands Quest, Nibelor 4: Berserk Brewery
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("Come back when you do.", cid)
			end
			npcHandler.topic[cid] = 0
		end
	end
	if msgcontains(msg, "buy animal cure") or msgcontains(msg, "animal cure") then -- animal cure for in service of yalahar
		if player:getStorageValue(Storage.InServiceofYalahar.Questline) >= 30 and player:getStorageValue(Storage.InServiceofYalahar.Questline) <= 54 then
			npcHandler:say("You want to buy animal cure for 400 gold coins?", cid)
			npcHandler.topic[cid] = 13
		else
			npcHandler:say("Im out of stock.", cid)
		end
	elseif msgcontains(msg, "yes") and npcHandler.topic[cid] == 13 then
		if npcHandler.topic[cid] == 13 and player:removeMoneyNpc(400) then
			player:addItem(9734, 1)
			npcHandler:say("Here you go.", cid)
			npcHandler.topic[cid] = 0
		else
			npcHandler:say("You dont have enough of gold coins.", cid)
			npcHandler.topic[cid] = 0
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
