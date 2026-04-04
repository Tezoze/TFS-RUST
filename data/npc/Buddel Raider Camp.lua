-- Buddel Raider Camp - Converted from XML to Lua NpcType
-- Original XML: data/npc/Buddel Raider Camp.xml
-- Original Script: data/npc/scripts/Buddel Raider Camp.lua

local npcId = "Buddel Raider Camp"  -- Unique ID for spawn system
local npcDisplayName = "Buddel"  -- Name shown in-game
local npcType = Game.createNpcType(npcId)

-- NPC Properties (from XML)
npcType:name(npcDisplayName)
npcType:nameDescription("a buddel")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 143, lookHead = 19, lookBody = 57, lookLegs = 22, lookFeet = 20})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Shop items (from XML parameters)
local shopItems = {
    {id = 11213, buy = 0, sell = 45, subType = 0, name = "i"},
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

-- Per-NPC handler storage to prevent state sharing between multiple instances
local npcHandlers = {}

local function getHandlers(npc)
	local npcId = npc:getId()
	if not npcHandlers[npcId] then
		npcHandlers[npcId] = {
			keywordHandler = KeywordHandler:new(),
			npcHandler = nil
		}
		npcHandlers[npcId].npcHandler = NpcHandler:new(npcHandlers[npcId].keywordHandler)
		
		local handler = npcHandlers[npcId].npcHandler
		local kh = npcHandlers[npcId].keywordHandler
		
		-- Travel
		local travelNode = kh:addKeyword({'svargrond'}, StdModule.say, {npcHandler = handler, text = 'Give me |TRAVELCOST| and I bring you to Svargrond. Alright?', cost = 50, discount = 'postman'})
			travelNode:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = handler, premium = false, cost = 50, discount = 'postman', destination = Position(32255, 31197, 7) })
			travelNode:addChildKeyword({'no'}, StdModule.say, {npcHandler = handler, reset = true, text = 'SHIP AHOY! I AM BUDDEL THE ..... did you say no??? Alright.'})
		
		-- Trade
		local function openTradeWindow(cid, message, keywords, parameters, node)
			if not handler:isFocused(cid) then return false end
			local player = Player(cid)
			if not player then return false end
			local currentNpc = Npc(getNpcCid())
			local shopList = {}
			for _, item in ipairs(shopItems) do
				table.insert(shopList, {id = item.id, buy = item.buy, sell = item.sell, subType = item.subType or 0, name = item.name})
			end
			currentNpc:openShopWindow(player, shopList, function() return true end, function() return true end)
			handler:say('Take all the time you need to browse my wares.', cid)
			return true
		end
		kh:addKeyword({'trade'}, openTradeWindow, {npcHandler = handler})
		
		handler:addModule(FocusModule:new())
	end
	return npcHandlers[npcId]
end

-- NpcType callbacks (MUST call setCurrentNpc first!)
npcType:eventType(NPCS_EVENT_APPEAR)
npcType:onAppear(function(npc, creature)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onCreatureAppear(creature)
end)

npcType:eventType(NPCS_EVENT_DISAPPEAR)
npcType:onDisappear(function(npc, creature)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onCreatureDisappear(creature)
end)

npcType:eventType(NPCS_EVENT_SAY)
npcType:onSay(function(npc, creature, type, message)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onCreatureSay(creature, type, message)
end)

npcType:eventType(NPCS_EVENT_THINK)
npcType:onThink(function(npc, interval)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onThink()
end)

npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
npcType:onCloseChannel(function(npc, creature)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onPlayerCloseChannel(creature)
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

npcType:register()
