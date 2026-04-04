-- Gnomission - Converted from XML to Lua NpcType
-- Original XML: data/npc/Gnomission.xml
-- Original Script: data/npc/scripts/Gnomission.lua

local npcName = "Gnomission"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a gnomission")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 493, lookHead = 41, lookBody = 115, lookLegs = 100, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if(not npcHandler:isFocused(cid)) then
		return false
	end
	local player = Player(cid)

	if(msgcontains(msg, "warzones")) then
		npcHandler:say({
			"There are three warzones. In each warzone you will find fearsome foes. At the end you'll find their mean master. The masters is well protected though. ...",
			"Make sure to talk to our gnomish agent in there for specifics of its' protection. ...",
			"Oh, and to be able to enter the second warzone you have to best the first. To enter the third you have to best the second. ...",
			"And you can enter each one only once every twenty hours. Your normal teleport crystals won't work on these teleporters. You will have to get mission crystals from Gnomally."
		}, cid)
		npcHandler.topic[cid] = 1
	elseif(msgcontains(msg, "job")) then
		npcHandler:say("I am responsible for our war {missions}, to {trade} with seasoned soldiers and rewarding war {heroes}. You have to be rank 4 to enter the {warzones}.", cid)
		npcHandler.topic[cid] = 2
	elseif(msgcontains(msg, "heroes")) then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say({
				"You can trade special spoils of war to get a permission to use the war teleporters to the area of the corresponding boss without need of mission crystals. ...",
				"Which one would you like to trade: the deathstrike's {snippet}, gnomevil's {hat} or the abyssador {lash}?"
			}, cid)
			npcHandler.topic[cid] = 3
		end
	elseif(msgcontains(msg, "snippet")) then
		if npcHandler.topic[cid] == 3 then
			if player:getStorageValue(Storage.BigfootBurden.QuestLine) < 30 then
				npcHandler:say("It seems you did not even set one big foot into the warzone, I am sorry.")
			else
				if player:getStorageValue(Storage.BigfootBurden.Warzone1Access) < 1 then
					if player:removeItem(18430, 1) then
						player:setStorageValue(Storage.BigfootBurden.Warzone1Access, 1)
						npcHandler:say("As a war hero you are allowed to use the warzone teleporter one for free!", cid)
						npcHandler.topic[cid] = 0
					else
						npcHandler:say("I can't let you enter the warzone teleporter one for free, unless you handle me a Deathstrike's snippet. But can still always use a red teleport crystal.", cid)
					end
				else
					npcHandler:say("We've already talked about that.", cid)
				end
			end
		end
	elseif(msgcontains(msg, "lash")) then
		if npcHandler.topic[cid] == 3 then
			if player:getStorageValue(Storage.BigfootBurden.QuestLine) < 30 then
				npcHandler:say("It seems you did not even set one big foot into the warzone, I am sorry.")
			else
				if player:getStorageValue(Storage.BigfootBurden.Warzone3Access) < 1 then 
					if player:getStorageValue(Storage.BigfootBurden.WarzoneStatus) >= 3 then
						if player:removeItem(18496, 1) then
							player:setStorageValue(Storage.BigfootBurden.Warzone3Access, 1)
							npcHandler:say("As a war hero you are allowed to use the warzone teleporter three for free!", cid)
							npcHandler.topic[cid] = 0
						else
							npcHandler:say("I can't let you enter the warzone teleporter two for free, unless you handle me an Abyssador's lash. But can still always use a red teleport crystal.", cid)
						end
					else
						npcHandler:say("You need to defeat the first warzone boss to be able to get free access to the second warzone.", cid)
					end
				else
					npcHandler:say("We've already talked about that.", cid)
				end
			end
		end
	elseif(msgcontains(msg, "hat")) then
		if npcHandler.topic[cid] == 3 then
			if player:getStorageValue(Storage.BigfootBurden.QuestLine) < 30 then
				npcHandler:say("It seems you did not even set one big foot into the warzone, I am sorry.")
			else
				if player:getStorageValue(Storage.BigfootBurden.Warzone2Access) < 1 then
					if player:getStorageValue(Storage.BigfootBurden.WarzoneStatus) >= 2 then
						if player:removeItem(18495, 1) then
							player:setStorageValue(Storage.BigfootBurden.Warzone2Access, 1)
							npcHandler:say("As a war hero you are allowed to use the warzone teleporter second for free!", cid)
							npcHandler.topic[cid] = 0
						else
							npcHandler:say("I can't let you enter the warzone teleporter three for free, unless you handle me a Gnomevil's hat. But can still always use a red teleport crystal.", cid)
						end
					else
						npcHandler:say("You need to defeat the second warzone boss to be able to get free access to the third warzone.", cid)
					end
				else
					npcHandler:say("We've already talked about that.", cid)
				end
			end
		end
	elseif(msgcontains(msg, "mission")) then
		if player:getStorageValue(Storage.BigfootBurden.QuestLine) >= 30 then
			if player:getStorageValue(Storage.BigfootBurden.WarzoneStatus) < 1 then
				npcHandler:say("Fine, I grant you the permission to enter the warzones. Be warned though, this will be not a picnic. Better bring some friends with you. Bringing a lot of them sounds like a good idea.", cid)
				player:setStorageValue(Storage.BigfootBurden.WarzoneStatus, 1)
			else
				npcHandler:say("You have already accepted this mission.", cid)
			end
			npcHandler.topic[cid] = 0
		else
			npcHandler:say("Sorry, you have not yet earned enough renown that we would risk your life in such a dangerous mission.", cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

local function onTradeRequest(cid)
	if Player(cid):getStorageValue(Storage.BigfootBurden.bossKills) < 20 then
		npcHandler:say('Only if you have killed 20 of our major enemies in the warzones I am allowed to trade with you.', cid)
		return false
	end

	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setCallback(CALLBACK_ONTRADEREQUEST, onTradeRequest)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 5791, buy = 0, sell = 6000, subType = 0, name = "stuffed dragon"},
    {id = 2361, buy = 0, sell = 20000, subType = 0, name = "frozen starlight"},
    {id = 6561, buy = 0, sell = 20000, subType = 0, name = "ceremonial ankh"},
    {id = 2453, buy = 0, sell = 42000, subType = 0, name = "arcane staff"},
    {id = 6528, buy = 0, sell = 42000, subType = 0, name = "the avenger"},
    {id = 5803, buy = 0, sell = 42000, subType = 0, name = "arbalest"},
    {id = 2418, buy = 0, sell = 1000, subType = 0, name = "golden sickle"},
    {id = 2130, buy = 0, sell = 2000, subType = 0, name = "golden amulet"},
    {id = 2131, buy = 0, sell = 500, subType = 0, name = "star amulet"},
    {id = 8849, buy = 0, sell = 10000, subType = 0, name = "modified crossbow"},
    {id = 2435, buy = 0, sell = 1500, subType = 0, name = "dwarven axe"},
    {id = 10529, buy = 0, sell = 10000, subType = 0, name = "sea serpent trophy"},
    {id = 12635, buy = 0, sell = 7500, subType = 0, name = "souleater trophy"},
    {id = 7964, buy = 0, sell = 5000, subType = 0, name = "marlin trophy"},
    {id = 1986, buy = 0, sell = 2000, subType = 0, name = "red tome"},
    {id = 1982, buy = 0, sell = 2000, subType = 0, name = "purple tome"},
    {id = 7417, buy = 0, sell = 45000, subType = 0, name = "runed sword"},
    {id = 6103, buy = 0, sell = 30000, subType = 0, name = "unholy book"},
    {id = 2184, buy = 0, sell = 10000, subType = 0, name = "crystal wand"},
    {id = 5080, buy = 0, sell = 30000, subType = 0, name = "panda teddy"},
    {id = 7183, buy = 0, sell = 20000, subType = 0, name = "baby seal doll"},
    {id = 10532, buy = 0, sell = 20000, subType = 0, name = "bejeweled ship's telescope"},
    {id = 8853, buy = 0, sell = 50000, subType = 0, name = "the ironworker"},
    {id = 7453, buy = 0, sell = 55000, subType = 0, name = "executioner"},
    {id = 10523, buy = 0, sell = 15000, subType = 0, name = "egg of the many"},
    {id = 10309, buy = 0, sell = 15000, subType = 0, name = "claw of 'the noxious spawn'"},
    {id = 12649, buy = 0, sell = 60000, subType = 0, name = "blade of corruption"},
    {id = 7416, buy = 0, sell = 30000, subType = 0, name = "bloody edge"},
    {id = 2493, buy = 0, sell = 40000, subType = 0, name = "demon helmet"},
    {id = 2504, buy = 0, sell = 40000, subType = 0, name = "dwarven legs"},
    {id = 8857, buy = 0, sell = 12000, subType = 0, name = "silkweaver bow"},
    {id = 2524, buy = 0, sell = 1500, subType = 0, name = "ornamented shield"},
    {id = 7730, buy = 0, sell = 15000, subType = 0, name = "blue legs"},
    {id = 18397, buy = 150, sell = 0, subType = 0, name = "mushroom pie"},
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
    -- Quest check: must have killed 20 warzone bosses
    if player:getStorageValue(Storage.BigfootBurden.bossKills) < 20 then
        npcHandler:say('Only if you have killed 20 of our major enemies in the warzones I am allowed to trade with you.', cid)
        return false
    end
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
