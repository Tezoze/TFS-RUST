-- Ishina - Converted from XML to Lua NpcType
-- Original XML: data/npc/Ishina.xml
-- Original Script: data/npc/scripts/Ishina.lua

local npcName = "Ishina"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ishina")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 150, lookHead = 95, lookBody = 9, lookLegs = 87, lookFeet = 95})
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
		if player:getSex() == PLAYERSEX_MALE then
			npcHandler:say('My jewelled belt? <giggles> That\'s not very manly. Maybe you\'d prefer a scimitar like Habdel has.', cid)
			return true
		end

		if player:getStorageValue(Storage.OutfitQuest.Oriental.AddonHipwear) < 1 then
			npcHandler:say('My jewelled belt? Of course I could make one for you, but I have a small request. Would you fulfil a task for me?', cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, 'comb') then
		if player:getSex() == PLAYERSEX_MALE then
			npcHandler:say('Comb? This is a jewellery shop.', cid)
			return true
		end

		if player:getStorageValue(Storage.OutfitQuest.Oriental.AddonHipwear) == 1 then
			npcHandler:say('Have you brought me a mermaid\'s comb?', cid)
			npcHandler.topic[cid] = 3
		end
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				'Listen, um... I have been wanting a comb for a long time... not just any comb, but a mermaid\'s comb. Having a mermaid\'s comb means never having split ends again! ...',
				'You know what that means to a girl! Could you please bring me such a comb? I really would appreciate it.'
			}, cid)
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 2 then
			player:setStorageValue(Storage.OutfitQuest.DefaultStart, 1)
			player:setStorageValue(Storage.OutfitQuest.Oriental.AddonHipwear, 1)
			npcHandler:say('Yay! I will wait for you to return with a mermaid\'s comb then.', cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 then
			if not player:removeItem(5945, 1) then
				npcHandler:say('No... that\'s not it.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			player:setStorageValue(Storage.OutfitQuest.Oriental.AddonHipwear, 2)
			player:addOutfitAddon(150, 1)
			player:addOutfitAddon(146, 1)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
			npcHandler:say('Yeah! That\'s it! I can\'t wait to comb my hair! Oh - but first, I\'ll fulfil my promise: Here is your jewelled belt! Thanks again!', cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'no') and npcHandler.topic[cid] ~= 0 then
		npcHandler:say('Oh... okay.', cid)
		npcHandler.topic[cid] = 0
	end

	return true
end

keywordHandler:addKeyword({'need'}, StdModule.say, {npcHandler = npcHandler, text = 'I am a jeweller. Maybe you want to have a look at my wonderful {offers}.'})
keywordHandler:addKeyword({'offers'}, StdModule.say, {npcHandler = npcHandler, text = 'Well, I sell gems and {goblets}. If you\'d like to see my offers, ask me for a {trade}.'})
keywordHandler:addKeyword({'goblets'}, StdModule.say, {npcHandler = npcHandler, text = 'Ah, our newest import! We have golden goblets, silver goblets and bronze goblets. All of them have space for a hand-written dedication.'})

npcHandler:setMessage(MESSAGE_GREET, 'Be greeted, |PLAYERNAME|. Which of my fine gems do you {need}?')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Daraman\'s blessings and good bye.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Daraman\'s blessings and good bye.')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2144, buy = 0, sell = 280, subType = 0, name = "black pearl"},
    {id = 7632, buy = 0, sell = 3000, subType = 0, name = "green giant shimmering pearl"},
    {id = 7633, buy = 0, sell = 3000, subType = 0, name = "brown giant shimmering pearl"},
    {id = 9971, buy = 0, sell = 5000, subType = 0, name = "gold ingot"},
    {id = 2157, buy = 0, sell = 850, subType = 0, name = "gold nugget"},
    {id = 2150, buy = 0, sell = 200, subType = 0, name = "small amethyst"},
    {id = 2145, buy = 0, sell = 300, subType = 0, name = "small diamond"},
    {id = 2149, buy = 0, sell = 250, subType = 0, name = "small emerald"},
    {id = 7762, buy = 0, sell = 200, subType = 0, name = "small enchanted amethyst"},
    {id = 7761, buy = 0, sell = 250, subType = 0, name = "small enchanted emerald"},
    {id = 7760, buy = 0, sell = 250, subType = 0, name = "small enchanted ruby"},
    {id = 7759, buy = 0, sell = 250, subType = 0, name = "small enchanted sapphire"},
    {id = 2147, buy = 0, sell = 250, subType = 0, name = "small ruby"},
    {id = 2146, buy = 0, sell = 250, subType = 0, name = "small sapphire"},
    {id = 9970, buy = 0, sell = 200, subType = 0, name = "small topaz"},
    {id = 2121, buy = 0, sell = 100, subType = 0, name = "wedding ring"},
    {id = 2143, buy = 0, sell = 160, subType = 0, name = "white pearl"},
    {id = 2144, buy = 560, sell = 0, subType = 0, name = "black pearl"},
    {id = 5807, buy = 2000, sell = 0, subType = 0, name = "bronze goblet"},
    {id = 2130, buy = 6600, sell = 0, subType = 0, name = "golden amulet"},
    {id = 5805, buy = 5000, sell = 0, subType = 0, name = "golden goblet"},
    {id = 2133, buy = 3560, sell = 0, subType = 0, name = "ruby necklace"},
    {id = 5806, buy = 3000, sell = 0, subType = 0, name = "silver goblet"},
    {id = 2150, buy = 400, sell = 0, subType = 0, name = "small amethyst"},
    {id = 2145, buy = 600, sell = 0, subType = 0, name = "small diamond"},
    {id = 2149, buy = 500, sell = 0, subType = 0, name = "small emerald"},
    {id = 2147, buy = 500, sell = 0, subType = 0, name = "small ruby"},
    {id = 2146, buy = 500, sell = 0, subType = 0, name = "small sapphire"},
    {id = 2121, buy = 990, sell = 0, subType = 0, name = "wedding ring"},
    {id = 2143, buy = 320, sell = 0, subType = 0, name = "white pearl"},
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
