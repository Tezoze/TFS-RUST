-- Irmana - Converted from XML to Lua NpcType
-- Original XML: data/npc/Irmana.xml
-- Original Script: data/npc/scripts/Irmana.lua

local npcName = "Irmana"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a irmana")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 140, lookHead = 78, lookBody = 90, lookLegs = 13, lookFeet = 14, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



function creatureSayCallback(cid, type, msg)
	if(not npcHandler:isFocused(cid)) then
		return false
	end

	local player = Player(cid)

	if(msgcontains(msg, "addon")) then
		if(getPlayerStorageValue(cid, Storage.OutfitQuest.Nobleman.AddonHat) < 1) then
			npcHandler:say("Currently we are offering accessories for the nobleman - and, of course, noblewoman - outfit. Would you like to hear more about our offer?", cid)
			npcHandler.topic[cid] = 1
		elseif getPlayerStorageValue(cid, Storage.OutfitQuest.Nobleman.AddonOutfit) < 1 then
			npcHandler:say("Currently we are offering accessories for the nobleman - and, of course, noblewoman - outfit. Would you like to hear more about our offer?", cid)
			npcHandler.topic[cid] = 1
		else
			npcHandler:say("You have already bought the two addons.", cid)
		end

	elseif(msgcontains(msg, "yes")) then
		if npcHandler.topic[cid] == 1 then
			if player:getSex() == PLAYERSEX_FEMALE then
				npcHandler:say("Especially for you, mylady, we are offering a pretty {hat} and a beautiful {dress} like the ones I wear. Which one are you interested in?", cid)
			else
				npcHandler:say("Especially for you, my lord, we are offering a pretty {hat} and a noble {coat} like the ones nobles wear. Which one are you interested in?", cid)
			end
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 3 then
			if(doPlayerRemoveMoney(cid, 150000) and getPlayerStorageValue(cid, Storage.OutfitQuest.Nobleman.AddonHat) < 1) then
				npcHandler:say("Congratulations! Here is your brand-new accessory, I hope you like it. Please visit us again! ", cid)
				npcHandler.topic[cid] = 0
				player:addOutfitAddon(140, 2)
				player:addOutfitAddon(132, 2)
				setPlayerStorageValue(cid, Storage.OutfitQuest.Nobleman.AddonHat, 1)
				if player:getStorageValue(Storage.OutfitQuest.Nobleman.AddonHat) == 1 and player:getStorageValue(Storage.OutfitQuest.Nobleman.AddonOutfit) == 1 then
					player:addAchievement(226) -- Achievement Aristocrat
				end
			end
		elseif npcHandler.topic[cid] == 4 then
			if(doPlayerRemoveMoney(cid, 150000) and getPlayerStorageValue(cid, Storage.OutfitQuest.Nobleman.AddonOutfit) < 1) then
				npcHandler:say("Congratulations! Here is your brand-new accessory, I hope you like it. Please visit us again! ", cid)
				npcHandler.topic[cid] = 0
				player:addOutfitAddon(140, 1)
				player:addOutfitAddon(132, 1)
				setPlayerStorageValue(cid, Storage.OutfitQuest.Nobleman.AddonOutfit, 1)
				if player:getStorageValue(Storage.OutfitQuest.Nobleman.AddonHat) == 1 and player:getStorageValue(Storage.OutfitQuest.Nobleman.AddonOutfit) == 1 then
					player:addAchievement(226) -- Achievement Aristocrat
				end
			end
		elseif npcHandler.topic[cid] == 5 then
			 if getPlayerItemCount(cid,2655) >= 1 then
      			doPlayerRemoveItem(cid,2655,1)
				npcHandler:say("A {Red Robe}! Great. Here, take this red piece of cloth, I don\'t need it anyway.", cid)
				doPlayerAddItem(cid,5911,1)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say('Are you trying to mess with me?!', cid)
			end
		elseif npcHandler.topic[cid] == 6 then
   			 if getPlayerItemCount(cid,2663) >= 1 then
				doPlayerRemoveItem(cid,2663,1)
				npcHandler:say("A {Mystic Turban}! Great. Here, take this blue piece of cloth, I don\'t need it anyway.", cid)
				doPlayerAddItem(cid,5912,1)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say('Are you trying to mess with me?!', cid)
			end
		elseif npcHandler.topic[cid] == 7 then
   			 if getPlayerItemCount(cid,2652) >= 150 then
				doPlayerRemoveItem(cid,2652,150)
				npcHandler:say("A 150 {Green Tunic}! Great. Here, take this green piece of cloth, I don\'t need it anyway.", cid)
				doPlayerAddItem(cid,5910,1)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say('Are you trying to mess with me?!', cid)
			end
		end
	elseif(msgcontains(msg, "hat") or msgcontains(msg, "accessory")) and (npcHandler.topic[cid] == 2 and getPlayerStorageValue(cid, Storage.OutfitQuest.Nobleman.AddonHat) < 1) then
		selfSay("This accessory requires a small fee of 150000 gold pieces. Of course, we do not want to put you at any risk to be attacked while carrying this huge amount of money. ...", cid)
		selfSay("This is why we have established our brand-new instalment sale. You can choose to either pay the price at once, or if you want to be safe, by instalments of 10000 gold pieces. ...", cid)
		selfSay("I also have to inform you that once you started paying for one of the accessories, you have to finish the payment first before you can start paying for the other one, of course. ...", cid)
		npcHandler:say("Are you interested in purchasing this accessory?", cid)
		npcHandler.topic[cid] = 3
	elseif(msgcontains(msg, "dress") or msgcontains(msg, "coat")) and (npcHandler.topic[cid] == 2 and getPlayerStorageValue(cid, Storage.OutfitQuest.Nobleman.AddonOutfit) < 1) then
		selfSay("This accessory requires a small fee of 150000 gold pieces. Of course, we do not want to put you at any risk to be attacked while carrying this huge amount of money. ...", cid)
		selfSay("This is why we have established our brand-new instalment sale. You can choose to either pay the price at once, or if you want to be safe, by instalments of 10000 gold pieces. ...", cid)
		selfSay("I also have to inform you that once you started paying for one of the accessories, you have to finish the payment first before you can start paying for the other one, of course. ...", cid)
		npcHandler:say("Are you interested in purchasing this accessory?", cid)
		npcHandler.topic[cid] = 4
	elseif(msgcontains(msg, "red robe")) then
		npcHandler:say("Have you found a {Red Robe} for me?", cid)
		npcHandler.topic[cid] = 5
	elseif(msgcontains(msg, "mystic turban")) then
		npcHandler:say("Have you found a {Mystic Turban} for me?", cid)
		npcHandler.topic[cid] = 6
	elseif(msgcontains(msg, "green tunic")) then
		npcHandler:say("Have you found {150 Green Tunic} for me?", cid)
		npcHandler.topic[cid] = 7	
	
	end
	
	--Scatterbrained 
	
	if player:getStorageValue(45215) == 1 then
		if(msgcontains(msg, "hat"))  then
			npcHandler:say("Yes, I can help you with fabricating a {dark hat}", cid)			
		end 
		
		if(msgcontains(msg, "dark hat")) then
			npcHandler:say("To create a dark hat, I need one piece of {minotaur leather} and two {bat wings}. Do you have those materials with you by coincidence?", cid)	
			npcHandler.topic[cid] = 8
			
		elseif(msgcontains(msg, "yes")) and npcHandler.topic[cid] == 8 then
			if player:getItemCount(5878) >= 1 and player:getItemCount(5894) >= 2 then
				player:removeItem(5878, 1)
				player:removeItem(5894, 2)
				doPlayerAddItem(player,10046,1)
				player:setStorageValue(45215, 2) 
				npcHandler:say("A little stitch here and a little stitch there... perfect! Here you are. With the best wishes to your master.", cid)
			else
				npcHandler:say("To create a dark hat, I need {one} piece of minotaur leather and {two} bat wings. Do you have those materials with you by coincidence?", cid)	
			end
			npcHandler.topic[cid] = 9
		end
	end
	
	
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 5883, buy = 0, sell = 120, subType = 0, name = "Ape Fur"},
    {id = 11216, buy = 0, sell = 15, subType = 0, name = "Badger Fur"},
    {id = 12404, buy = 0, sell = 300, subType = 0, name = "Black Wool"},
    {id = 5912, buy = 0, sell = 200, subType = 0, name = "Blue Piece of Cloth"},
    {id = 5913, buy = 0, sell = 100, subType = 0, name = "Brown Piece of Cloth"},
    {id = 10606, buy = 0, sell = 30, subType = 0, name = "Bunch of Troll Hair"},
    {id = 12412, buy = 0, sell = 120, subType = 0, name = "Dirty Turban"},
    {id = 12640, buy = 0, sell = 20, subType = 0, name = "Downy Feather"},
    {id = 10575, buy = 0, sell = 160, subType = 0, name = "Frost Giant Pelt"},
    {id = 12414, buy = 0, sell = 80, subType = 0, name = "Geomancer's Robe"},
    {id = 10607, buy = 0, sell = 90, subType = 0, name = "Ghostly Tissue"},
    {id = 5877, buy = 0, sell = 100, subType = 0, name = "Green Dragon Leather"},
    {id = 5910, buy = 0, sell = 200, subType = 0, name = "Green Piece of Cloth"},
    {id = 12426, buy = 0, sell = 180, subType = 0, name = "Jewelled Belt"},
    {id = 10608, buy = 0, sell = 60, subType = 0, name = "Lion's Mane"},
    {id = 5876, buy = 0, sell = 150, subType = 0, name = "Lizard Leather"},
    {id = 5878, buy = 0, sell = 80, subType = 0, name = "Minotaur Leather"},
    {id = 12431, buy = 0, sell = 250, subType = 0, name = "Necromantic Robe"},
    {id = 12442, buy = 0, sell = 430, subType = 0, name = "Noble Turban"},
    {id = 11196, buy = 0, sell = 15, subType = 0, name = "Piece of Crocodile Leather"},
    {id = 12429, buy = 0, sell = 110, subType = 0, name = "Purple Robe"},
    {id = 5948, buy = 0, sell = 200, subType = 0, name = "Red Dragon Leather"},
    {id = 5911, buy = 0, sell = 300, subType = 0, name = "Red Piece of Cloth"},
    {id = 12448, buy = 0, sell = 66, subType = 0, name = "Rope Belt"},
    {id = 9958, buy = 0, sell = 1000, subType = 0, name = "Royal Tapestry"},
    {id = 12449, buy = 0, sell = 120, subType = 0, name = "Safety Pin"},
    {id = 11324, buy = 0, sell = 25, subType = 0, name = "Shaggy Tail"},
    {id = 11209, buy = 0, sell = 35, subType = 0, name = "Silky Fur"},
    {id = 2657, buy = 0, sell = 50, subType = 0, name = "Simple Dress"},
    {id = 11191, buy = 0, sell = 50, subType = 0, name = "Skunk Tail"},
    {id = 10611, buy = 0, sell = 400, subType = 0, name = "Snake Skin"},
    {id = 5886, buy = 0, sell = 1000, subType = 0, name = "Spool of Yarn"},
    {id = 11210, buy = 0, sell = 50, subType = 0, name = "Striped Fur"},
    {id = 10601, buy = 0, sell = 120, subType = 0, name = "Tattered Piece of Robe"},
    {id = 11224, buy = 0, sell = 150, subType = 0, name = "Thick Fur"},
    {id = 9837, buy = 0, sell = 800, subType = 0, name = "Velvet Tapestry"},
    {id = 11235, buy = 0, sell = 30, subType = 0, name = "Warwolf Fur"},
    {id = 11234, buy = 0, sell = 380, subType = 0, name = "Werewolf Fur"},
    {id = 5909, buy = 0, sell = 100, subType = 0, name = "White Piece of Cloth"},
    {id = 11212, buy = 0, sell = 20, subType = 0, name = "Winter Wolf Fur"},
    {id = 11236, buy = 0, sell = 15, subType = 0, name = "Wool"},
    {id = 5914, buy = 0, sell = 150, subType = 0, name = "Yellow Piece of Cloth"},
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
