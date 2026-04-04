-- Duncan - Converted from XML to Lua NpcType
-- Original XML: data/npc/Duncan.xml
-- Original Script: data/npc/scripts/Duncan.lua

local npcName = "Duncan"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a duncan")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 151, lookHead = 38, lookBody = 23, lookFeet = 116, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	local storage = Storage.OutfitQuest.PirateSabreAddon

	if isInArray({"outfit", "addon"}, msg) and player:getStorageValue(Storage.OutfitQuest.PirateBaseOutfit) == 1 then
		npcHandler:say("You're talking about my sabre? Well, even though you earned our trust, you'd have to fulfill a task first before you are granted to wear such a sabre.", cid)
	elseif msgcontains(msg, "task") then
		if player:getStorageValue(storage) < 1 then
			npcHandler:say("Are you up to the task which I'm going to give you and willing to prove you're worthy of wearing such a sabre?", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "eye patches") then
		if player:getStorageValue(storage) == 1 then
			npcHandler:say("Have you gathered 100 eye patches?", cid)
			npcHandler.topic[cid] = 3
		end
	elseif msgcontains(msg, "peg legs") then
		if player:getStorageValue(storage) == 2 then
			npcHandler:say("Have you gathered 100 peg legs?", cid)
			npcHandler.topic[cid] = 4
		end
	elseif msgcontains(msg, "hooks") then
		if player:getStorageValue(storage) == 3 then
			npcHandler:say("Have you gathered 100 hooks?", cid)
			npcHandler.topic[cid] = 5
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				"Listen, the task is not that hard. Simply prove that you are with us and not with the pirates from Nargor by bringingme some of their belongings. ...",
				"Bring me 100 of their eye patches, 100 of their peg legs and 100 of their hooks, in that order. ...",
				"Have you understood everything I told you and are willing to handle this task?"
			}, cid)
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 2 then
			player:setStorageValue(storage, 1)
			player:setStorageValue(Storage.OutfitQuest.DefaultStart, 1) --this for default start of Outfit and Addon Quests
			npcHandler:say("Good! Come back to me once you have gathered 100 eye patches.", cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 then
			if player:removeItem(6098, 100) then
				player:setStorageValue(storage, 2)
				npcHandler:say("Good job. Alright, now bring me 100 peg legs.", cid)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("You don't have it...", cid)
			end
		elseif npcHandler.topic[cid] == 4 then
			if player:removeItem(6126, 100) then
				player:setStorageValue(storage, 3)
				npcHandler:say("Nice. Lastly, bring me 100 pirate hooks. That should be enough to earn your sabre.", cid)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("You don't have it...", cid)
			end
		elseif npcHandler.topic[cid] == 5 then
			if player:removeItem(6097, 100) then
				player:setStorageValue(storage, 4)
				npcHandler:say("I see, I see. Well done. Go to Morgan and tell him this codeword: 'firebird'. He'll know what to do.", cid)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("You don't have it...", cid)
			end
		end
	elseif msgcontains(msg, "no") then
		if npcHandler.topic[cid] >= 1 then
			npcHandler:say("Then no.", cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 5616, buy = 40, sell = 0, subType = 0, name = "pirate tapestry"},
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
