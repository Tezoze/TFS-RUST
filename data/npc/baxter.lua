-- Baxter - Converted from XML to Lua NpcType
-- Original XML: data/npc/Baxter.xml
-- Original Script: data/npc/scripts/Baxter.lua

local npcName = "Baxter"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a baxter")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 131, lookHead = 77, lookBody = 29, lookLegs = 29, lookFeet = 115})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'People of Thais, bring honour to your king by fighting in the orc war!' },
	{ text = 'The orcs are preparing for war!!!' }
}


local function creatureSayCallback(cid, type, msg)

	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	
	
	--Cuadno responde Mission
	if msgcontains(msg, 'mission') then	
		if player:getStorageValue(45210) == -1 then
			npcHandler:say({
				'We\'ve got a rat problem in the sewers. In the name of our glorious king, I\'m paying 1 blinking piece of gold for every freshly killed rat you bring me. ...',
				'You seem strong! Do you want to help fighting the orcs? They prepare themselves for war! We need everyone who\'s capable of killing greenskins! Ask me about the {orc war} if you are interested.'
			}, cid)
			npcHandler.topic[cid] = 1  
		end
	end
	
	if msgcontains(msg, 'achievement') then	
		if player:getStorageValue(45210) == 2 then
			npcHandler:say({
				'Not bad, you will come a long way if you keep up the good work!',
				'or',
				'Salutations henchman/mercenary/orc butcher/war hero player!! Keep up the good work! Here you go.',
				'or, when advancing in ranks',
				'Good news everyone!! player will be promoted to the highest rank, a war hero! Congratulations!'
			}, cid)
			player:setStorageValue(45210, 3)
		end
	end
	
	if player:getStorageValue(45210) == 3 then
			npcHandler:say({
				'Burning war equipment in the orc fortrest'				
			}, cid)			
	end	
		
	
	if msgcontains(msg, 'war plans') then	
	
		if player:getStorageValue(45210) == 4 then
			npcHandler:say({
				'The orcs are moving their troops. We need to know what they are up to! Go to the orc fortress. There should be some kind of blackboard somewhere. Study it and tell me their plans!'				
			}, cid)
			player:setStorageValue(45210, 5)
		end
		if player:getStorageValue(45210) == 5 then
			npcHandler:say({
				'Find the Orc Plans!'				
			}, cid)			
		end
		if player:getStorageValue(45210) == 6 then
			npcHandler:say({
				'Great job!! I have to discuss their plans with the king immediately. Keep it up!'				
			}, cid)
			player:setStorageValue(45210, 7)
			player:addExperience(5000)
	end
	
	
	--Cuadno responde Mission
	if msgcontains(msg, 'orc war') and npcHandler.topic[cid] == 1 then			
		player:setStorageValue(45210, 1)
		npcHandler:say({
				'I hope you know where to find orc land!! There are various things that you can do for your country! First of all, we need to establish some outposts that need to be maintained. ...',
				'For that, you can buy bricklayer kits from me. Just tell me if you need some. Secondly, the orcs are building ballistae, catapults and siege towers for their attack! Set them on fire! ...',
				'Thirdly, we need to find out what they are planning. Go find the current war plans! Actually they are not THAT smart, so they don\'t change them very often. Ask me first before you are heading there for nothing! ...',
				'That would be all for the moment. Oh I forgot, you collect achievement points while you\'re on duty. Ask me regularly about them, you might get a promotion if you\'re doing well!'
			}, cid)
	
		npcHandler.topic[cid] = 2
	end

	if msgcontains(msg, 'where') and npcHandler.topic[cid] == 2 then
	npcHandler:say({
				'It\'s north east of here. Pass the mountains of Kazordoon and go east. Past the wyvern nest, you\'ll find our outpost and then you\'ll enter Ulderek\'s Rock. Take care!'
			}, cid)
	
	



		end
	end
	return true
end

npcHandler:addModule(VoiceModule:new(voices))

npcHandler:setMessage(MESSAGE_GREET, "LONG LIVE KING TIBIANUS!")
npcHandler:setMessage(MESSAGE_FAREWELL, "LONG LIVE THE KING!")
npcHandler:setMessage(MESSAGE_WALKAWAY, "LONG LIVE THE KING!")
npcHandler:setMessage(MESSAGE_SENDTRADE, "Do you bring freshly killed rats for a bounty of 1 gold each? By the way, I also buy orc teeth and other stuff you ripped from their bloody corp... I mean... well, you know what I mean.")


npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 12409, buy = 0, sell = 20, subType = 0, name = "broken helmet"},
    {id = 12408, buy = 0, sell = 35, subType = 0, name = "broken shamanic staff"},
    {id = 3813, buy = 0, sell = 1, subType = 0, name = "dead rat"},
    {id = 12435, buy = 0, sell = 30, subType = 0, name = "orc leather"},
    {id = 11113, buy = 0, sell = 150, subType = 0, name = "orc tooth"},
    {id = 12433, buy = 0, sell = 85, subType = 0, name = "orcish gear"},
    {id = 12434, buy = 0, sell = 45, subType = 0, name = "shamanic hood"},
    {id = 12436, buy = 0, sell = 80, subType = 0, name = "skull belt"},
    {id = 8613, buy = 100, sell = 0, subType = 0, name = "bricklayers kit"},
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
