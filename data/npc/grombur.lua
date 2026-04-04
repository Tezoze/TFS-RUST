-- Grombur - Converted from XML to Lua NpcType
-- Original XML: data/npc/Grombur.xml
-- Original Script: data/npc/scripts/Grombur.lua

local npcName = "Grombur"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a grombur")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 160, lookHead = 114, lookBody = 77, lookLegs = 79, lookFeet = 114})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, "nokmir") then
		if player:getStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll) == 2 then
			npcHandler:say("Oh well, I liked Nokmir. He used to be a good dwarf until that day on which he stole the ring from {Rerun}.", cid)
			npcHandler.topic[cid] = 1
		else
			npcHandler:say("DEBUG: Your JusticeForAll storage value is " .. player:getStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll) .. ". You need it to be 2 to talk about Nokmir.", cid)
		end
	elseif msgcontains(msg, "rerun") then
		if npcHandler.topic[cid] == 1 then
			player:setStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll, 3)
			npcHandler:say("Yeah, he's the lucky guy in this whole story. I heard rumours that emperor Rehal had plans to promote Nokmir, but after this whole thievery story, he might pick Rerun instead.", cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.hiddenCityOfBeregar.TheGoodGuard) < 1 then
			npcHandler:say("Got any dwarven brown ale?? I DON'T THINK SO....and Bolfana, the tavern keeper, won't sell you anything. I'm sure about that...she doesn't like humans... I tell you what, if you get me a cask of dwarven brown ale, I allow you to enter the mine. Alright?", cid)
			npcHandler.topic[cid] = 2
		elseif player:getStorageValue(Storage.hiddenCityOfBeregar.TheGoodGuard) == 1 and player:removeItem(9689, 1) then
			player:setStorageValue(Storage.hiddenCityOfBeregar.TheGoodGuard, 2)
			player:setStorageValue(Storage.hiddenCityOfBeregar.DoorSouthMine, 1)
			npcHandler:say("HOW?....WHERE?....AHHHH, I don't mind....SLUUUUUURP....tastes a little flat but I had worse. Thank you. Just don't tell anyone that I let you in.", cid)
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 2 then
			player:setStorageValue(Storage.hiddenCityOfBeregar.TheGoodGuard, 1)
			player:setStorageValue(Storage.hiddenCityOfBeregar.DefaultStart, 1)
			npcHandler:say("Haha, fine! Don't waste time and get me the ale. See you.", cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_WALKAWAY, "See you my friend.")
npcHandler:setMessage(MESSAGE_FAREWELL, "See you my friend.")
npcHandler:setMessage(MESSAGE_GREET, "STOP RIGHT THERE!..... Oh, just a human. What's up big guy?")
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


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

npcHandler:addModule(FocusModule:new())
npcType:register()
