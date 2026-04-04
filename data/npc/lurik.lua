-- Lurik - Converted from XML to Lua NpcType
-- Original XML: data/npc/Lurik.xml
-- Original Script: data/npc/scripts/Lurik.lua

local npcName = "Lurik"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a lurik")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 133, lookHead = 38, lookBody = 94, lookLegs = 96, lookFeet = 116, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	local player = Player(cid)
	if not npcHandler:isFocused(cid) then
		return false
	end

	if msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.ExplorerSociety.TheAstralPortals) == 56 and player:getStorageValue(Storage.ExplorerSociety.QuestLine) == 56 then
			npcHandler:say("Ah, you've just come in time. An experienced explorer is just what we need here! Would you like to go on a mission for us?", cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(Storage.ExplorerSociety.TheIslandofDragons) == 58 and player:getStorageValue(Storage.ExplorerSociety.QuestLine) == 58 then
			if player:removeItem(7314, 1) then
				npcHandler:say({
					"A frozen dragon lord? This is just the information we needed! And you even brought a scale from it! Take these 5000 gold pieces as a reward. ...",
					"As you did such a great job, I might have another mission for you later."
				}, cid)
				player:addItem(2152, 50)
				player:setStorageValue(Storage.ExplorerSociety.TheIslandofDragons, 59)
				player:setStorageValue(Storage.ExplorerSociety.QuestLine, 59)
			else
				npcHandler:say("You're not done yet...", cid)
			end
		elseif player:getStorageValue(Storage.ExplorerSociety.TheIslandofDragons) == 59 and player:getStorageValue(Storage.ExplorerSociety.QuestLine) == 59 then
			npcHandler:say({
				"Ah, yes, the mission. Let me tell you about something called ice music. ...",
				"There is a cave on Hrodmir, north of the southernmost barbarian camp Krimhorn. ...",
				"In this cave, there are a waterfall and a lot of stalagmites. ...",
				"When the wind blows into this cave and hits the stalagmites, it is supposed to create a sound similar to a soft song. ...",
				"Please take this resonance crystal and use it on the stalagmites in the cave to record the sound of the wind."
			}, cid)
			player:setStorageValue(Storage.ExplorerSociety.TheIceMusic, 60)
			player:setStorageValue(Storage.ExplorerSociety.QuestLine, 60)
			player:addItem(7242, 1)
		elseif player:getStorageValue(Storage.ExplorerSociety.TheIceMusic) == 61 and player:getStorageValue(Storage.ExplorerSociety.QuestLine) == 61 and player:removeItem(7315, 1) then
			npcHandler:say({
				"Ah! You did it! I can't wait to hear the sound... but I will do that in a silent moment. ...",
				"You helped as much in our research here. As a reward, you may use our astral portal in the upper room from now on. ...",
				"For just one orichalcum pearl, you can travel between Liberty Bay and Svargrond. Thank you again!"
			}, cid)
			npcHandler.topic[cid] = 0
			player:setStorageValue(Storage.ExplorerSociety.TheIceMusic, 62)
			player:setStorageValue(Storage.ExplorerSociety.QuestLine, 62)
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 32 then
			npcHandler:say({
				"You are the one who became an honorary barbarian! The one who made friends with the grim local musher and helped the shamans of Nibelor! The one they call old bearhugg ... erm ... I mean indeed I might have a mission for someone like you ...",
				"We are trying to find out what is happening in the raider camps. Through our connection to the shamans we could get a covered contact in their majorcamp far to the south. We equipped our contact with a memory crystal so he could report all he knew ...",
				"We need you to recover this crystal. Travel to the southern camp of the raiders and find our contact man there. Get the memory crystal and bring ithere. The society and the shamans will then decide our next steps. Do you think you can do this?"
			}, cid)
			npcHandler.topic[cid] = 2
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 33 then
			npcHandler:say("Have you retrieved the memory crystal?", cid)
			npcHandler.topic[cid] = 3
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 34 and player:getStorageValue(Storage.TheIceIslands.MemoryCrystal) > os.time() then
			npcHandler:say("Give me some more time!", cid)
			npcHandler.topic[cid] = 0
		elseif player:getStorageValue(Storage.TheIceIslands.Questline) == 34 and player:getStorageValue(Storage.TheIceIslands.MemoryCrystal) < os.time() then
			npcHandler:say({
				"The information was quite useful. What worries me most are not the raiders but those that have driven them from the old mines...",
				"We need to investigate the mines. Most entrances collapsed due to the lack of maintenance but there should be some possibilities to get in ...",
				"In case you find a door, Ill tell you the old trick of the Carlin mining company to open it <whisper> <whisper>. Find some hint or someone who is willing to talk about what is going on there."
			}, cid)
			npcHandler.topic[cid] = 0
			player:setStorageValue(Storage.TheIceIslands.Questline, 35)
			player:setStorageValue(Storage.TheIceIslands.Mission09, 1) -- Questlog The Ice Islands Quest, Formorgar Mines 1: The Mission
		end
	elseif msgcontains(msg, "yes") then
		-- ISLAND OF DRAGONS
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				"Now we're talking! Maybe you've already heard of the island Okolnir south of Hrodmir. ...",
				"Okolnir is the home of a new and fierce dragon race, the so-called frost dragons. However, we have no idea where they originate from. ...",
				"Rumours say that dragon lords, that roamed on this isle, were somehow turned into frost dragons when the great frost covered Okolnir. ...",
				"Travel to Okolnir and try to find a proof for the existence of dragon lords there in the old times. I think old Buddel might be able to bring you there."
			}, cid)
			npcHandler.topic[cid] = 0
			player:setStorageValue(Storage.ExplorerSociety.TheIslandofDragons, 57)
			player:setStorageValue(Storage.ExplorerSociety.QuestLine, 57)
		-- ISLAND OF DRAGONS
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say("Excellent. Just report about your mission when you got the memory crystal.", cid)
			player:setStorageValue(Storage.TheIceIslands.Questline, 33)
			player:setStorageValue(Storage.TheIceIslands.Mission08, 2) -- Questlog The Ice Islands Quest, The Contact
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 then
			if player:removeItem(7281, 1) then
				npcHandler:say("Ah, great. Please give me some time to evaluate the information. Then talk to me again about your mission. ", cid)
				player:setStorageValue(Storage.TheIceIslands.Questline, 34)
				player:setStorageValue(Storage.TheIceIslands.Mission08, 4) -- Questlog The Ice Islands Quest, The Contact
				player:setStorageValue(Storage.TheIceIslands.MemoryCrystal, os.time() + 5 * 60)
				npcHandler.topic[cid] = 0
			end
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 5022, buy = 80, sell = 0, subType = 0, name = "orichalcum pearl"},
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
