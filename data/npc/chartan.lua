-- Chartan - Converted from XML to Lua NpcType
-- Original XML: data/npc/Chartan.xml
-- Original Script: data/npc/scripts/Chartan.lua

local npcName = "Chartan"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a chartan")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 338})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.WrathoftheEmperor.Questline) == 2 then
			npcHandler:say("Mhm, what are you doing here. Who zent you? ", cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(Storage.WrathoftheEmperor.Questline) == 3 then
			npcHandler:say("Zo are you ready to get zomezing done?", cid)
			npcHandler.topic[cid] = 2
		elseif player:getStorageValue(Storage.WrathoftheEmperor.Questline) == 5 then
			npcHandler:say("Zo? Did you find a way to reztore ze teleporter? ", cid)
			npcHandler.topic[cid] = 3
		end
	elseif msgcontains(msg, "zalamon") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				"I zee. Zalamon zent word of ze arrival of a zoftzkin quite zome time ago. Zat muzt be you zen. ... ",
				"Well, I exzpected zomeone more - imprezzive. However, we will zee how far you can get. You've got newz from ze zouz? ... ",
				"Hm, I underztand. ... ",
				"Oh you did. ... ",
				"I zee. Interezting. ... ",
				"You being here meanz we have eztablished connectionz to ze zouz. Finally. And you are going to help uz. Well, zere iz zertainly a lot for you to do. Zo better get ztarted. "
			}, cid)
			player:setStorageValue(Storage.WrathoftheEmperor.Questline, 3)
			player:setStorageValue(Storage.WrathoftheEmperor.Mission01, 3) --Questlog, Wrath of the Emperor "Mission 01: Catering the Lions Den"
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say({
				"Alright. Well, az you might not be aware of it yet - we are on top of an old temple complex. It haz been abandoned and it haz crumbled over time. ...",
				"Ze teleporter over zere uzed to work juzt fine to get uz back to ze zouz. But it haz ztopped operating for quite zome time. ... ",
				"My men believe it iz a dizturbanze cauzed by ze corruption zat zpreadz everywhere. Zey are too zcared to go down zere. And zat'z where you come in. ... ",
				"Zere were meanz to activate teleporterz zomewhere in ze complex. But zinze you cannot reach all ze roomz, I guezz you will have to improvize. ... ",
				"Here iz ze key to ze entranze to ze complex. Figure zomezing out, reztore ze teleporter zo we can get back to ze plainz in ze zouz. "
			}, cid)
			player:setStorageValue(Storage.WrathoftheEmperor.Questline, 4)
			player:setStorageValue(Storage.WrathoftheEmperor.Mission02, 1) --Questlog, Wrath of the Emperor "Mission 02: First Contact"
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 then
			npcHandler:say({
				"You did it! Zere waz zome kind of zparkle and I zink it iz working again - oh pleaze feel free to try it, I uhm, I will wait here and be ready juzt in caze zomezing uhm happenz to you. ... ",
				"And if you head to Zalamon, be zure to inform him about our zituation. Food rationz are running low and we are ztill not well equipped. We need to eztablish a working zupply line. "
			}, cid)
			player:setStorageValue(Storage.WrathoftheEmperor.Questline, 6)
			player:setStorageValue(Storage.WrathoftheEmperor.Mission02, 3) --Questlog, Wrath of the Emperor "Mission 02: First Contact"
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 7634, buy = 0, sell = 5, subType = 0, name = "empty potion flask"},
    {id = 7635, buy = 0, sell = 5, subType = 0, name = "empty potion flask"},
    {id = 7636, buy = 0, sell = 5, subType = 0, name = "empty potion flask"},
    {id = 2006, buy = 0, sell = 5, subType = 0, name = "vial"},
    {id = 2274, buy = 45, sell = 0, subType = 0, name = "avalanche rune"},
    {id = 2260, buy = 10, sell = 0, subType = 0, name = "blank rune"},
    {id = 2261, buy = 15, sell = 0, subType = 0, name = "destroy field rune"},
    {id = 12638, buy = 5, sell = 0, subType = 0, name = "dragonfruit"},
    {id = 2695, buy = 3, sell = 0, subType = 0, name = "egg"},
    {id = 2671, buy = 10, sell = 0, subType = 0, name = "ham"},
    {id = 2277, buy = 38, sell = 0, subType = 0, name = "energy field rune"},
    {id = 2279, buy = 85, sell = 0, subType = 0, name = "energy wall rune"},
    {id = 2313, buy = 31, sell = 0, subType = 0, name = "explosion rune"},
    {id = 2305, buy = 117, sell = 0, subType = 0, name = "fire bomb rune"},
    {id = 2301, buy = 28, sell = 0, subType = 0, name = "fire field rune"},
    {id = 2303, buy = 61, sell = 0, subType = 0, name = "fire wall rune"},
    {id = 2304, buy = 45, sell = 0, subType = 0, name = "great fireball rune"},
    {id = 7591, buy = 190, sell = 0, subType = 0, name = "great health potion"},
    {id = 7590, buy = 120, sell = 0, subType = 0, name = "great mana potion"},
    {id = 8472, buy = 190, sell = 0, subType = 0, name = "great spirit potion"},
    {id = 7618, buy = 34, sell = 0, subType = 0, name = "health potion"},
    {id = 2311, buy = 12, sell = 0, subType = 0, name = "heavy magic missile rune"},
    {id = 2265, buy = 95, sell = 0, subType = 0, name = "intense healing rune"},
    {id = 2287, buy = 4, sell = 0, subType = 0, name = "light magic missile rune"},
    {id = 7620, buy = 50, sell = 0, subType = 0, name = "mana potion"},
    {id = 2285, buy = 21, sell = 0, subType = 0, name = "poison field rune"},
    {id = 2289, buy = 52, sell = 0, subType = 0, name = "poison wall rune"},
    {id = 2292, buy = 12, sell = 0, subType = 0, name = "stalagmite rune"},
    {id = 7588, buy = 75, sell = 0, subType = 0, name = "strong health potion"},
    {id = 7589, buy = 80, sell = 0, subType = 0, name = "strong mana potion"},
    {id = 2268, buy = 108, sell = 0, subType = 0, name = "sudden death rune"},
    {id = 2273, buy = 175, sell = 0, subType = 0, name = "ultimate healing rune"},
    {id = 8473, buy = 310, sell = 0, subType = 0, name = "ultimate health potion"},
    {id = 18304, buy = 20, sell = 0, subType = 0, name = "crystalline arrow"},
    {id = 18435, buy = 20, sell = 0, subType = 0, name = "prismatic bolt"},
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
