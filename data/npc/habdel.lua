-- Habdel - Converted from XML to Lua NpcType
-- Original XML: data/npc/Habdel.xml
-- Original Script: data/npc/scripts/Habdel.lua

local npcName = "Habdel"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a habdel")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 146, lookHead = 95, lookBody = 1, lookFeet = 58, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, 'outfit') then
		if player:getSex() == PLAYERSEX_FEMALE then
			npcHandler:say('My scimitar? Well, mylady, I do not want to sound rude, but I don\'t think a scimitar would fit to your beautiful outfit. If you are looking for an accessory, why don\'t you talk to Ishina?', cid)
			return true
		end
		if player:getStorageValue(Storage.OutfitQuest.firstOrientalAddon) < 1 then
			npcHandler:say('My scimitar? Yes, that is a true masterpiece. Of course I could make one for you, but I have a small request. Would you fulfil a task for me?', cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, 'comb') then			
		if player:getSex() == PLAYERSEX_FEMALE then
			npcHandler:say('Comb? This is a weapon shop.', cid)
			return true
		end		
		if player:getStorageValue(Storage.OutfitQuest.firstOrientalAddon) == 1 then
			npcHandler:say('Have you brought a mermaid\'s comb for Ishina?', cid)
			npcHandler.topic[cid] = 3
		end
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				'Listen, um... I know that Ishina has been wanting a comb for a long time... not just any comb, but a mermaid\'s comb. She said it prevents split ends... or something. ...',
				'Do you think you could get one for me so I can give it to her? I really would appreciate it. {yes/no}'
			}, cid)
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 2 then
			player:setStorageValue(Storage.OutfitQuest.DefaultStart, 1)
			player:setStorageValue(Storage.OutfitQuest.firstOrientalAddon, 1)
			npcHandler:say('Brilliant! I will wait for you to return with a mermaid\'s comb then.', cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 then
			if not player:removeItem(5945, 1) then
				npcHandler:say('No... that\'s not it.', cid)
				npcHandler.topic[cid] = 0
				return true
			end
			player:setStorageValue(Storage.OutfitQuest.firstOrientalAddon, 2)
			player:addOutfitAddon(150, 1)
			player:addOutfitAddon(146, 1)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
			npcHandler:say('Yeah! That\'s it! I can\'t wait to give it to her! Oh - but first, I\'ll fulfil my promise: Here is your scimitar! Thanks again!', cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'no') and npcHandler.topic[cid] ~= 0 then
		npcHandler:say('Ah well. Doesn\'t matter.', cid)
		npcHandler.topic[cid] = 0
	end
	return true
end

keywordHandler:addKeyword({'weapons'}, StdModule.say, {npcHandler = npcHandler, text = 'I sell the finest weapons in town. If you\'d like to see my offers, ask me for a {trade}.'})

npcHandler:setMessage(MESSAGE_GREET, 'Welcome |PLAYERNAME|! See the fine {weapons} I sell.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Good bye. Come back soon.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Good bye. Come back soon.')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2386, buy = 0, sell = 7, subType = 0, name = "axe"},
    {id = 2378, buy = 0, sell = 80, subType = 0, name = "battle axe"},
    {id = 2417, buy = 0, sell = 120, subType = 0, name = "battle hammer"},
    {id = 2449, buy = 0, sell = 5, subType = 0, name = "bone club"},
    {id = 2450, buy = 0, sell = 20, subType = 0, name = "bone sword"},
    {id = 2395, buy = 0, sell = 118, subType = 0, name = "carlin sword"},
    {id = 2382, buy = 0, sell = 1, subType = 0, name = "club"},
    {id = 2416, buy = 0, sell = 50, subType = 0, name = "crowbar"},
    {id = 2379, buy = 0, sell = 2, subType = 0, name = "dagger"},
    {id = 2387, buy = 0, sell = 260, subType = 0, name = "double axe"},
    {id = 2485, buy = 0, sell = 3, subType = 0, name = "doublet"},
    {id = 2525, buy = 0, sell = 100, subType = 0, name = "dwarven shield"},
    {id = 2392, buy = 0, sell = 1000, subType = 0, name = "fire sword"},
    {id = 2381, buy = 0, sell = 400, subType = 0, name = "halberd"},
    {id = 2380, buy = 0, sell = 4, subType = 0, name = "hand axe"},
    {id = 2388, buy = 0, sell = 25, subType = 0, name = "hatchet"},
    {id = 2412, buy = 0, sell = 35, subType = 0, name = "katana"},
    {id = 2397, buy = 0, sell = 51, subType = 0, name = "longsword"},
    {id = 2398, buy = 0, sell = 30, subType = 0, name = "mace"},
    {id = 2394, buy = 0, sell = 100, subType = 0, name = "morning star"},
    {id = 2428, buy = 0, sell = 350, subType = 0, name = "orcish axe"},
    {id = 2384, buy = 0, sell = 5, subType = 0, name = "rapier"},
    {id = 2385, buy = 0, sell = 12, subType = 0, name = "sabre"},
    {id = 2406, buy = 0, sell = 10, subType = 0, name = "short sword"},
    {id = 2405, buy = 0, sell = 3, subType = 0, name = "sickle"},
    {id = 2559, buy = 0, sell = 5, subType = 0, name = "small axe"},
    {id = 2383, buy = 0, sell = 240, subType = 0, name = "spike sword"},
    {id = 2448, buy = 0, sell = 10, subType = 0, name = "studded club"},
    {id = 2376, buy = 0, sell = 25, subType = 0, name = "sword"},
    {id = 2410, buy = 0, sell = 2, subType = 0, name = "throwing knife"},
    {id = 2377, buy = 0, sell = 450, subType = 0, name = "two handed sword"},
    {id = 2391, buy = 0, sell = 470, subType = 0, name = "war hammer"},
    {id = 2386, buy = 20, sell = 0, subType = 0, name = "axe"},
    {id = 2378, buy = 235, sell = 0, subType = 0, name = "battle axe"},
    {id = 2417, buy = 350, sell = 0, subType = 0, name = "battle hammer"},
    {id = 2450, buy = 75, sell = 0, subType = 0, name = "bone sword"},
    {id = 2395, buy = 473, sell = 0, subType = 0, name = "carlin sword"},
    {id = 2382, buy = 5, sell = 0, subType = 0, name = "club"},
    {id = 2651, buy = 8, sell = 0, subType = 0, name = "coat"},
    {id = 2416, buy = 260, sell = 0, subType = 0, name = "crowbar"},
    {id = 2379, buy = 5, sell = 0, subType = 0, name = "dagger"},
    {id = 2380, buy = 8, sell = 0, subType = 0, name = "hand axe"},
    {id = 2397, buy = 160, sell = 0, subType = 0, name = "longsword"},
    {id = 2398, buy = 90, sell = 0, subType = 0, name = "mace"},
    {id = 2394, buy = 430, sell = 0, subType = 0, name = "morning star"},
    {id = 2384, buy = 15, sell = 0, subType = 0, name = "rapier"},
    {id = 2385, buy = 35, sell = 0, subType = 0, name = "sabre"},
    {id = 2406, buy = 26, sell = 0, subType = 0, name = "short sword"},
    {id = 2405, buy = 7, sell = 0, subType = 0, name = "sickle"},
    {id = 2383, buy = 8000, sell = 0, subType = 0, name = "spike sword"},
    {id = 2376, buy = 25, sell = 0, subType = 0, name = "sword"},
    {id = 2410, buy = 25, sell = 0, subType = 0, name = "throwing knife"},
    {id = 2399, buy = 42, sell = 0, subType = 0, name = "throwing star"},
    {id = 2377, buy = 950, sell = 0, subType = 0, name = "two handed sword"},
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
