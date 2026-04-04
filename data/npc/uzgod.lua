-- Uzgod - Converted from XML to Lua NpcType
-- Original XML: data/npc/Uzgod.xml
-- Original Script: data/npc/scripts/Uzgod.lua

local npcName = "Uzgod"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a uzgod")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 160, lookHead = 96, lookBody = 60, lookLegs = 97, lookFeet = 116})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if(not npcHandler:isFocused(cid)) then
		return false
	end
	local player = Player(cid)
	if(msgcontains(msg, "piece of draconian steel")) then
		npcHandler:say("You bringing me draconian steel and obsidian lance in exchange for obsidian knife?", cid)
		npcHandler.topic[cid] = 15
	elseif(msgcontains(msg, "yes") and npcHandler.topic[cid] == 15) then
		if player:getItemCount(5889) >= 1 and player:getItemCount(2425) >= 1 then
			if player:removeItem(5889, 1) and player:removeItem(2425, 1) then
				npcHandler:say("Here you have it.", cid)
				player:addItem(5908, 1)
				npcHandler.topic[cid] = 0
			end
		else
			npcHandler:say("You don\'t have these items.", cid)
			npcHandler.topic[cid] = 0
		end
	end

	if(msgcontains(msg, "pickaxe")) then
		if player:getStorageValue(Storage.ExplorerSociety.JoiningtheExplorers) == 1 and player:getStorageValue(Storage.ExplorerSociety.QuestLine) == 1 then
			npcHandler:say("True dwarven pickaxes having to be maded by true weaponsmith! You wanting to get pickaxe for explorer society?", cid)
			npcHandler.topic[cid] = 1
		end
	elseif(msgcontains(msg, "crimson sword")) then
		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission05) == 1 then
			npcHandler:say("Me don't sell crimson sword.", cid)
			npcHandler.topic[cid] = 5
		end
	elseif(msgcontains(msg, "forge")) then
		if(npcHandler.topic[cid] == 5) then
			npcHandler:say("You telling me to forge one?! Especially for you? You making fun of me?", cid)
			npcHandler.topic[cid] = 6
		end
	elseif(msgcontains(msg, "brooch")) then
		if player:getStorageValue(Storage.ExplorerSociety.JoiningtheExplorers) == 2 and player:getStorageValue(Storage.ExplorerSociety.QuestLine) == 2 then
			npcHandler:say("You got me brooch?", cid)
			npcHandler.topic[cid] = 3
		end
	elseif(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 1) then
			npcHandler:say("Me order book quite full is. But telling you what: You getting me something me lost and Uzgod seeing that your pickaxe comes first. Jawoll! You interested?", cid)
			npcHandler.topic[cid] = 2
		elseif(npcHandler.topic[cid] == 2) then
			npcHandler:say("Good good. You listening: Me was stolen valuable heirloom. Brooch from my family. Good thing is criminal was caught. Bad thing is, criminal now in dwarven prison of dwacatra is and must have taken brooch with him ...", cid)
			npcHandler:say("To get into dwacatra you having to get several keys. Each key opening way to other key until you get key to dwarven prison ...", cid)
			npcHandler:say("Last key should be in the generals quarter near armory. Only General might have key to enter there too. But me not knowing how to enter Generals private room at barracks. You looking on your own ...", cid)
			npcHandler:say("When got key, then you going down to dwarven prison and getting me that brooch. Tell me that you got brooch when having it.", cid)
			npcHandler.topic[cid] = 0
			player:setStorageValue(Storage.ExplorerSociety.JoiningtheExplorers, 2)
			player:setStorageValue(Storage.ExplorerSociety.QuestLine, 2)
		elseif(npcHandler.topic[cid] == 3) then
			if player:removeItem(4845, 1) then -----
				npcHandler:say("Thanking you for brooch. Me guessing you now want your pickaxe?", cid)
				npcHandler.topic[cid] = 4
			end
		elseif(npcHandler.topic[cid] == 4) then
			npcHandler:say("Here you have it.", cid)
			player:addItem(4874, 1) -----
			player:setStorageValue(Storage.ExplorerSociety.JoiningtheExplorers, 3)
			player:setStorageValue(Storage.ExplorerSociety.QuestLine, 3)
			npcHandler.topic[cid] = 0
		elseif(npcHandler.topic[cid] == 8) then
			if player:removeMoneyNpc(300) then
				npcHandler:say("Uhm. Ok, me do this... cheap just like you wanted it <ploing>... here you have it!", cid)
				player:addItem(7385, 1)
				player:setStorageValue(Storage.TheTravellingTraderQuest.Mission05, 2)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("You don't have enough gold.", cid)
				npcHandler.topic[cid] = 0
			end
		elseif(npcHandler.topic[cid] == 9) then
			if player:getMoney() + player:getBankBalance() >= 250 and player:getItemCount(5880) >= 3 then
				if player:removeMoneyNpc(250) and player:removeItem(5880, 3) then
					npcHandler:say("Ah, that's how me like me customers. Ok, me do this... <pling pling> ... another fine swing of the hammer here and there... <ploing>... here you have it!", cid)
					player:addItem(7385, 1)
					player:setStorageValue(Storage.TheTravellingTraderQuest.Mission05, 2)
					npcHandler.topic[cid] = 0
				end
			end
		end
	elseif(msgcontains(msg, "no")) then
		if(npcHandler.topic[cid] == 6) then
			npcHandler:say("Well. Thinking about it, me a smith, so why not. 1000 gold for your personal crimson sword. Ok?", cid)
			npcHandler.topic[cid] = 7
		elseif(npcHandler.topic[cid] == 7) then
			npcHandler:say("Too expensive?! You think me work is cheap? Well, if you want cheap, I can make cheap. Hrmpf. I make cheap sword for 300 gold. Ok?", cid)
			npcHandler.topic[cid] = 8
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, "Hiho |PLAYERNAME|! Wanna weapon, eh?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Farewell, |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Farewell, |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_SENDTRADE, "Of course, just browse through my wares. If you're only interested in {distance} equipment, let me know.")


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2377, buy = 0, sell = 190, subType = 0, name = "two handed sword"},
    {id = 2379, buy = 0, sell = 1, subType = 0, name = "dagger"},
    {id = 2383, buy = 0, sell = 225, subType = 0, name = "spike sword"},
    {id = 2384, buy = 0, sell = 3, subType = 0, name = "rapier"},
    {id = 2385, buy = 0, sell = 5, subType = 0, name = "sabre"},
    {id = 2392, buy = 0, sell = 1000, subType = 0, name = "fire sword"},
    {id = 2395, buy = 0, sell = 118, subType = 0, name = "carlin sword"},
    {id = 2397, buy = 0, sell = 51, subType = 0, name = "longsword"},
    {id = 2391, buy = 0, sell = 470, subType = 0, name = "war hammer"},
    {id = 2394, buy = 0, sell = 100, subType = 0, name = "morning star"},
    {id = 2398, buy = 0, sell = 23, subType = 0, name = "mace"},
    {id = 2417, buy = 0, sell = 50, subType = 0, name = "battle hammer"},
    {id = 2378, buy = 0, sell = 75, subType = 0, name = "battle axe"},
    {id = 2381, buy = 0, sell = 310, subType = 0, name = "halberd"},
    {id = 2387, buy = 0, sell = 260, subType = 0, name = "double axe"},
    {id = 2428, buy = 0, sell = 350, subType = 0, name = "orcish axe"},
    {id = 2389, buy = 0, sell = 1, subType = 0, name = "spear"},
    {id = 2377, buy = 950, sell = 0, subType = 0, name = "two handed sword"},
    {id = 2379, buy = 5, sell = 0, subType = 0, name = "dagger"},
    {id = 2384, buy = 15, sell = 0, subType = 0, name = "rapier"},
    {id = 2385, buy = 35, sell = 0, subType = 0, name = "sabre"},
    {id = 2394, buy = 430, sell = 0, subType = 0, name = "morning star"},
    {id = 2398, buy = 90, sell = 0, subType = 0, name = "mace"},
    {id = 2417, buy = 350, sell = 0, subType = 0, name = "battle hammer"},
    {id = 2378, buy = 235, sell = 0, subType = 0, name = "battle axe"},
    {id = 2389, buy = 10, sell = 0, subType = 0, name = "spear"},
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