-- Sandra - Converted from XML to Lua NpcType
-- Original XML: data/npc/Sandra.xml
-- Original Script: data/npc/scripts/Sandra.lua

local npcName = "Sandra"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a sandra")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 140, lookHead = 115, lookBody = 95, lookLegs = 125, lookFeet = 57, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'Great spirit potions as well as health and mana potions in different sizes!' },
	{ text = 'If you need alchemical fluids like slime and blood, get them here.' }
}

npcHandler:addModule(VoiceModule:new(voices))

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if isInArray({"vial", "ticket", "bonus", "deposit"}, msg) then
		if player:getStorageValue(Storage.OutfitQuest.MageSummoner.AddonBelt) < 1 then
			npcHandler:say("You have "..player:getStorageValue(Storage.OutfitQuest.MageSummoner.VialDepositCredits).." credits. We have a special offer right now for depositing vials. Are you interested in hearing it?", cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(Storage.OutfitQuest.MageSummoner.AddonBelt) >= 1 then
			npcHandler:say("Would you like to get a lottery ticket instead of the deposit for your vials?", cid)
			npcHandler.topic[cid] = 3
		end
	elseif msgcontains(msg, "prize") then
		npcHandler:say("Are you here to claim a prize?", cid)
		npcHandler.topic[cid] = 4
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				"The Edron academy has introduced a bonus system. Each time you deposit 100 vials without claiming the money for it, you will receive a lottery ticket. ...",
				"Some of these lottery tickets will grant you a special potion belt accessory, if you bring the ticket to me. ...",
				"If you join the bonus system now, I will ask you each time you are bringing back 100 or more vials to me whether you claim your deposit or rather want a lottery ticket. ...",
				"Of course, you can leave or join the bonus system at any time by just asking me for the 'bonus'. ...",
				"Would you like to join the bonus system now?"
			}, cid)
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say("Great! I've signed you up for our bonus system. From now on, you will have the chance to win the potion belt addon!", cid)
			player:setStorageValue(Storage.OutfitQuest.MageSummoner.AddonBelt, 1)
			player:setStorageValue(Storage.OutfitQuest.DefaultStart, 1) --this for default start of Outfit and Addon Quests
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 then
			if player:getStorageValue(Storage.OutfitQuest.MageSummoner.VialDepositCredits) >= 100 or player:removeItem(7634, 100) or player:removeItem(7635, 100) or player:removeItem(7636, 100) then
				npcHandler:say("Alright, thank you very much! Here is your lottery ticket, good luck. Would you like to deposit more vials that way?", cid)
				player:setStorageValue(Storage.OutfitQuest.MageSummoner.VialDepositCredits, player:getStorageValue(Storage.OutfitQuest.MageSummoner.VialDepositCredits)-100);
				player:addItem(5957, 1)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("Sorry, but you don't have 100 empty flasks or vials of the SAME kind and thus don't qualify for the lottery. Would you like to deposit the vials you have as usual and receive 5 gold per vial?", cid)
				npcHandler.topic[cid] = 0
			end
		elseif npcHandler.topic[cid] == 4 then
			if player:getStorageValue(Storage.OutfitQuest.MageSummoner.AddonBelt) == 1 and player:removeItem(5958, 1) then
				npcHandler:say("Congratulations! Here, from now on you can wear our lovely potion belt as accessory.", cid)
				player:setStorageValue(Storage.OutfitQuest.MageSummoner.AddonBelt, 2)
				player:addOutfitAddon(138, 1)
				player:addOutfitAddon(133, 1)
				player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
			else
				npcHandler:say("Sorry, but you don't have your lottery ticket with you.... or not signed in bonus system", cid)
			end
			npcHandler.topic[cid] = 0
		end
		return true
	end
end

keywordHandler:addKeyword({'shop'}, StdModule.say, {npcHandler = npcHandler, text = 'I sell potions and fluids. If you\'d like to see my offers, ask me for a {trade}.'})

npcHandler:setMessage(MESSAGE_GREET, "Hello |PLAYERNAME|, welcome to the fluid and potion {shop} of Edron.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye, |PLAYERNAME|, please come back soon.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye, |PLAYERNAME|, please come back soon.")
npcHandler:setMessage(MESSAGE_SENDTRADE, "Of course, just browse through my wares. By the way, if you'd like to join our bonus system for depositing flasks and vial, you have to tell me about that {deposit}.")

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 7634, buy = 0, sell = 5, subType = 0, name = "empty potion flask"},
    {id = 7635, buy = 0, sell = 5, subType = 0, name = "empty potion flask"},
    {id = 7636, buy = 0, sell = 5, subType = 0, name = "empty potion flask"},
    {id = 2006, buy = 0, sell = 5, subType = 0, name = "vial"},
    {id = 2006, buy = 10, sell = 0, subType = 13, name = "vial of urine"},
    {id = 2006, buy = 10, sell = 0, subType = 2, name = "vial of blood"},
    {id = 2006, buy = 10, sell = 0, subType = 11, name = "vial of oil"},
    {id = 2006, buy = 10, sell = 0, subType = 1, name = "vial of water"},
    {id = 7591, buy = 190, sell = 0, subType = 0, name = "great health potion"},
    {id = 7590, buy = 120, sell = 0, subType = 0, name = "great mana potion"},
    {id = 8472, buy = 190, sell = 0, subType = 0, name = "great spirit potion"},
    {id = 7618, buy = 34, sell = 0, subType = 0, name = "health potion"},
    {id = 7620, buy = 50, sell = 0, subType = 0, name = "mana potion"},
    {id = 7588, buy = 75, sell = 0, subType = 0, name = "strong health potion"},
    {id = 7589, buy = 80, sell = 0, subType = 0, name = "strong mana potion"},
    {id = 8473, buy = 310, sell = 0, subType = 0, name = "ultimate health potion"},
}

-- Helper function to find shop item by id and subType (for fluid containers)
local function getShopItem(itemId, subType)
    local itemType = ItemType(itemId)
    if itemType:isFluidContainer() then
        for _, item in ipairs(shopItems) do
            if item.id == itemId and item.subType == subType then
                return item
            end
        end
    else
        for _, item in ipairs(shopItems) do
            if item.id == itemId then
                return item
            end
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
