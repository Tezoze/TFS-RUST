-- Alesar - Converted from XML to Lua NpcType
-- Original XML: data/npc/Alesar.xml
-- Original Script: data/npc/scripts/Alesar.lua

local npcName = "Alesar"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a alesar")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 80})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	local missionProgress = player:getStorageValue(Storage.DjinnWar.EfreetFaction.Mission02) or -1
	if msgcontains(msg, 'mission') then
		if player:getStorageValue(Storage.DjinnWar.EfreetFaction.Mission01) == 3 then
			if missionProgress < 1 then
				npcHandler:say({
					'So Baa\'leal thinks you are up to do a mission for us? ...',
					'I think he is getting old, entrusting human scum such as you are with an important mission like that. ...',
					'Personally, I don\'t understand why you haven\'t been slaughtered right at the gates. ...',
					'Anyway. Are you prepared to embark on a dangerous mission for us?'
				}, cid)
				npcHandler.topic[cid] = 1

			elseif isInArray({1, 2}, missionProgress) then
				npcHandler:say('Did you find the tear of Daraman?', cid)
				npcHandler.topic[cid] = 2
			else
				npcHandler:say('Don\'t forget to talk to Malor concerning your next mission.', cid)
			end
		end

	elseif npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'yes') then
			npcHandler:say({
				'All right then, human. Have you ever heard of the {\'Tears of Daraman\'}? ...',
				'They are precious gemstones made of some unknown blue mineral and possess enormous magical power. ...',
				'If you want to learn more about these gemstones don\'t forget to visit our library. ...',
				'Anyway, one of them is enough to create thousands of our mighty djinn blades. ...',
				'Unfortunately my last gemstone broke and therefore I\'m not able to create new blades anymore. ...',
				'To my knowledge there is only one place where you can find these gemstones - I know for a fact that the Marid have at least one of them. ...',
				'Well... to cut a long story short, your mission is to sneak into Ashta\'daramai and to steal it. ...',
				'Needless to say, the Marid won\'t be too eager to part with it. Try not to get killed until you have delivered the stone to me.'
			}, cid)
			player:setStorageValue(Storage.DjinnWar.EfreetFaction.Mission02, 1)

		elseif msgcontains(msg, 'no') then
			npcHandler:say('Then not.', cid)
		end
		npcHandler.topic[cid] = 0

	elseif npcHandler.topic[cid] == 2 then
		if msgcontains(msg, 'yes') then
			if player:getItemCount(2346) == 0 or missionProgress ~= 2 then
				npcHandler:say('As I expected. You haven\'t got the stone. Shall I explain your mission again?', cid)
				npcHandler.topic[cid] = 1
			else
				npcHandler:say({
					'So you have made it? You have really managed to steal a Tear of Daraman? ...',
					'Amazing how you humans are just impossible to get rid of. Incidentally, you have this character trait in common with many insects and with other vermin. ...',
					'Nevermind. I hate to say it, but it you have done us a favour, human. That gemstone will serve us well. ...',
					'Baa\'leal, wants you to talk to Malor concerning some new mission. ...',
					'Looks like you have managed to extended your life expectancy - for just a bit longer.'
				}, cid)
				player:removeItem(2346, 1)
				player:setStorageValue(Storage.DjinnWar.EfreetFaction.Mission02, 3)
				npcHandler.topic[cid] = 0
			end

		elseif msgcontains(msg, 'no') then
			npcHandler:say('As I expected. You haven\'t got the stone. Shall I explain your mission again?', cid)
			npcHandler.topic[cid] = 1
		end
	end
	return true
end

local function onTradeRequest(cid)
	local player = Player(cid)
	
	if (player:getStorageValue(Storage.DjinnWar.EfreetFaction.Mission03) or -1) ~= 3 then
		npcHandler:say('I\'m sorry, but you don\'t have Malor\'s permission to trade with me.', cid)
		return false
	end

	return true
end

npcHandler:setMessage(MESSAGE_GREET, 'What do you want from me, |PLAYERNAME|?')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Finally.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Finally.')
npcHandler:setMessage(MESSAGE_SENDTRADE, 'At your service, just browse through my wares.')

npcHandler:setCallback(CALLBACK_ONTRADEREQUEST, onTradeRequest)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

local focusModule = FocusModule:new()
focusModule:addGreetMessage('hi')
focusModule:addGreetMessage('hello')
focusModule:addGreetMessage('djanni\'hah')
npcHandler:addModule(focusModule)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2535, buy = 0, sell = 900, subType = 0, name = "ancient shield"},
    {id = 2529, buy = 0, sell = 800, subType = 0, name = "black shield"},
    {id = 7428, buy = 0, sell = 10000, subType = 0, name = "bonebreaker"},
    {id = 2489, buy = 0, sell = 400, subType = 0, name = "dark armor"},
    {id = 2490, buy = 0, sell = 250, subType = 0, name = "dark helmet"},
    {id = 2434, buy = 0, sell = 2000, subType = 0, name = "dragon hammer"},
    {id = 7419, buy = 0, sell = 15000, subType = 0, name = "dreaded cleaver"},
    {id = 7860, buy = 0, sell = 2000, subType = 0, name = "earth knight axe"},
    {id = 7875, buy = 0, sell = 2000, subType = 0, name = "energy knight axe"},
    {id = 7750, buy = 0, sell = 2000, subType = 0, name = "fiery knight axe"},
    {id = 2393, buy = 0, sell = 17000, subType = 0, name = "giant sword"},
    {id = 7407, buy = 0, sell = 8000, subType = 0, name = "haunted blade"},
    {id = 7769, buy = 0, sell = 2000, subType = 0, name = "icy knight axe"},
    {id = 2476, buy = 0, sell = 5000, subType = 0, name = "knight armor"},
    {id = 2430, buy = 0, sell = 2000, subType = 0, name = "knight axe"},
    {id = 2477, buy = 0, sell = 5000, subType = 0, name = "knight legs"},
    {id = 2663, buy = 0, sell = 150, subType = 0, name = "mystic turban"},
    {id = 7421, buy = 0, sell = 22000, subType = 0, name = "onyx flail"},
    {id = 7411, buy = 0, sell = 20000, subType = 0, name = "ornamented axe"},
    {id = 2411, buy = 0, sell = 50, subType = 0, name = "poison dagger"},
    {id = 2419, buy = 0, sell = 150, subType = 0, name = "scimitar"},
    {id = 2409, buy = 0, sell = 900, subType = 0, name = "serpent sword"},
    {id = 2436, buy = 0, sell = 6000, subType = 0, name = "skull staff"},
    {id = 2479, buy = 0, sell = 500, subType = 0, name = "strange helmet"},
    {id = 7413, buy = 0, sell = 4000, subType = 0, name = "titan axe"},
    {id = 2528, buy = 0, sell = 8000, subType = 0, name = "tower shield"},
    {id = 2534, buy = 0, sell = 15000, subType = 0, name = "vampire shield"},
    {id = 2475, buy = 0, sell = 5000, subType = 0, name = "warrior helmet"},
    {id = 2535, buy = 5000, sell = 0, subType = 0, name = "ancient shield"},
    {id = 2489, buy = 1500, sell = 0, subType = 0, name = "dark armor"},
    {id = 2490, buy = 1000, sell = 0, subType = 0, name = "dark helmet"},
    {id = 2396, buy = 5000, sell = 0, subType = 0, name = "ice rapier"},
    {id = 2409, buy = 6000, sell = 0, subType = 0, name = "serpent sword"},
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
    -- Quest check: must have Malor's permission (Efreet Faction Mission03 = 3)
    if (player:getStorageValue(Storage.DjinnWar.EfreetFaction.Mission03) or -1) ~= 3 then
        npcHandler:say('I\'m sorry, but you don\'t have Malor\'s permission to trade with me.', cid)
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

npcType:register()
