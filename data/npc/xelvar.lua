-- Xelvar - Converted from XML to Lua NpcType
-- Original XML: data/npc/Xelvar.xml
-- Original Script: data/npc/scripts/Xelvar.lua

local npcName = "Xelvar"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a xelvar")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 70})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if not player then
		return false
	end

	if msgcontains(msg, "adventures") or msgcontains(msg, "join") then
		if player:getStorageValue(Storage.BigfootBurden.QuestLine) < 1 then
			npcHandler:say({
				"I am glad to hear that. In the spirit of our own foreign legion we suggested the gnomes might hire heroes like you to build some kind of troop. They gave me that strange crystal to allow people passage to their realm. ...",
				"I hereby grant you permission to use the basic gnomish teleporters. I also give you four gnomish teleport crystals. One will be used up each time you use the teleporter. ...",
				"You can stock up your supply by buying more from me. Just ask me for a {trade}. Gnomette in the teleport chamber of the gnome outpost will sell them too. ...",
				"The teleporter here will transport you to one of the bigger gnomish outposts. ...",
				"There you will meet Gnomerik, the recruitment officer of the Gnomes. If you are lost, Gnomette in the teleport chamber might be able to help you with directions. ...",
				"Good luck to you and don't embarrass your race down there! Keep in mind that you are a representative of the big people."
			}, cid)

			player:setStorageValue(Storage.BigfootBurden.QuestLine, 1)
			player:addItem(18457, 4)

			--npcHandler:say("Right now I am sort of {recruiting} people.", cid)
			npcHandler.topic[cid] = 1
			else npcHandler:say("You already talked with me.", cid)
		end
	elseif msgcontains(msg, "recruiting") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("Ok, so listen. Your help is needed. That is if you're the hero type. Our ... {partners} need some help in urgent matters.", cid)
			npcHandler.topic[cid] = 2
		end
	elseif msgcontains(msg, "partners") then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say("I guess the time of secrecy is over now. Well, we have an old alliance with another underground dwelling race, the {gnomes}.", cid)
			npcHandler.topic[cid] = 3
		end
	elseif msgcontains(msg, "gnomes") then
		if npcHandler.topic[cid] == 3 then
			npcHandler:say({
				"The gnomes preferred to keep our alliance and their whole {existence} a secret. They are a bit distrustful of others. ...",
				"They are quite self-sufficient and the fact that they are actually accepting some help is more than alarming. The gnomes are in real trouble and I am kind of an ambassador to find some people willing to {help}."
			}, cid)
			npcHandler.topic[cid] = 4
		end
	elseif msgcontains(msg, "help") then
		if npcHandler.topic[cid] == 4 then
			npcHandler:say({
				"The gnomes are locked in a war with an enemy that thins out their resources but foremost their manpower. We have suggested that people like you could be just the specialists they are looking for. ...",
				"If you are interested to {join} the gnomish cause I can arrange a meeting with their recruiter."
			}, cid)
			npcHandler.topic[cid] = 5
		end
	elseif msgcontains(msg, "join") then
		if npcHandler.topic[cid] == 5 then
			npcHandler:say({
				"I am glad to hear that. In the spirit of our own foreign legion we suggested the gnomes might hire heroes like you to build some kind of troop. They gave me that strange crystal to allow people passage to their realm. ...",
				"I hereby grant you permission to use the basic gnomish teleporters. I also give you four gnomish teleport crystals. One will be used up each time you use the teleporter. ...",
				"You can stock up your supply by buying more from me. Just ask me for a {trade}. Gnomette in the teleport chamber of the gnome outpost will sell them too. ...",
				"The teleporter here will transport you to one of the bigger gnomish outposts. ...",
				"There you will meet Gnomerik, the recruitment officer of the Gnomes. If you are lost, Gnomette in the teleport chamber might be able to help you with directions. ...",
				"Good luck to you and don't embarrass your race down there! Keep in mind that you are a representative of the big people."
			}, cid)

			player:setStorageValue(Storage.BigfootBurden.QuestLine, 1)
			player:addItem(18457, 4)
			npcHandler.topic[cid] = 0



	return true

end
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
end
end


-- Shop items (from XML parameters)
local shopItems = {
    {id = 18457, buy = 150, sell = 0, subType = 0, name = "teleport crystal"},
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
