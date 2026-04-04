-- Todd - Converted from XML to Lua NpcType
-- Original XML: data/npc/Todd.xml
-- Original Script: data/npc/scripts/Todd.lua

local npcName = "Todd"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a todd")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 115, lookLegs = 67, lookFeet = 114})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	if msgcontains(msg, "interesting") then
		npcHandler:say({
			"I'd really like to rebuild my reputation someday and maybe find a nice girl. If you come across scrolls of heroic deeds or addresses of lovely maidens... let me know! ...",
			"Oh no, it doesn't matter what name is on the scrolls. I'm, uhm... flexible! And money - yes, I can pay. My, erm... uncle died recently and left me a pretty sum. Yes."
		}, cid)
	end
end

keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = "I am... a traveler. Just leave me alone if you have nothing {interesting} to talk about."})
keywordHandler:addKeyword({'want'}, StdModule.say, {npcHandler = npcHandler, text = "I am... a traveler. Just leave me alone if you have nothing {interesting} to talk about."})
keywordHandler:addKeyword({'head'}, StdModule.say, {npcHandler = npcHandler, text = "Uhhh ohhhh one of the beers yesterday must have been bad."})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = "My Name? I am To... ahm... hum... My name is {Hugo}."})
keywordHandler:addKeyword({'hugo'}, StdModule.say, {npcHandler = npcHandler, text = "Yes, that's my name of course."})
keywordHandler:addKeyword({'todd'}, StdModule.say, {npcHandler = npcHandler, text = "Uh .. I... I met a Todd on the road. He told me he was traveling to Venore, look there for your Todd."})
keywordHandler:addKeyword({'thais'}, StdModule.say, {npcHandler = npcHandler, text = "I love that city."})
keywordHandler:addKeyword({'carlin'}, StdModule.say, {npcHandler = npcHandler, text = "I never was there. Now leave me alone."})
keywordHandler:addKeyword({'resistance'}, StdModule.say, {npcHandler = npcHandler, text = "Resistance is futile... uhm... I wonder where I picked that saying up. Oh my head..."})
keywordHandler:addKeyword({'money'}, StdModule.say, {npcHandler = npcHandler, text = "I don't know anything about money, missing or not."})
keywordHandler:addKeyword({'eclesius'}, StdModule.say, {npcHandler = npcHandler, text = "He often comes here. But his constant confusion gives me a worse headache than Frodo's beer. I rather avoid him."})
keywordHandler:addKeyword({'karl'}, StdModule.say, {npcHandler = npcHandler, text = "Uhm, never heard about him... and you can't prove otherwise."})
keywordHandler:addKeyword({'william'}, StdModule.say, {npcHandler = npcHandler, text = "Thats a common name, perhaps I met a William, not sure about that."})

npcHandler:setMessage(MESSAGE_GREET, "Uhm oh hello |PLAYERNAME|... not so loud please... my {head}... What ... do you {want}?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Yes, goodbye |PLAYERNAME|, just leave me alone.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Silence at last.")

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 12466, buy = 0, sell = 230, subType = 0, name = "scroll of heroic deeds"},
    {id = 12406, buy = 0, sell = 480, subType = 0, name = "small notebook"},
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
