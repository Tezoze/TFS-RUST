-- Gnomewart - Converted from XML to Lua NpcType
-- Original XML: data/npc/Gnomewart.xml
-- Original Script: data/npc/scripts/Gnomewart.lua

local npcName = "Gnomewart"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a gnomewart")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 493, lookHead = 41, lookBody = 100, lookLegs = 100, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, "endurance") then
		if player:getStorageValue(Storage.BigfootBurden.QuestLine) == 15 then
			npcHandler:say({
				"Ah, the test is a piece of mushroomcake! Just take the teleporter over there in the south and follow the hallway. ...",
				"You'll need to run quite a bit. It is important that you don't give up! Just keep running and running and running and ... I guess you got the idea. ...",
				"At the end of the hallway you'll find a teleporter. Step on it and you are done! I'm sure you'll do a true gnomerun! Afterwards talk to me."
			}, cid)
			player:setStorageValue(Storage.BigfootBurden.QuestLine, 17)
		elseif player:getStorageValue(Storage.BigfootBurden.QuestLine) == 17 then
			npcHandler:say("Just take the teleporter over there to the south and follow the hallway. At the end of the hallway you'll find a teleporter. Step on it and you are done!", cid)
		elseif player:getStorageValue(Storage.BigfootBurden.QuestLine) == 18 then
			npcHandler:say("You have passed the test and are ready to create your soul melody. Talk to Gnomelvis in the east about it.", cid)
			player:setStorageValue(Storage.BigfootBurden.QuestLine, 19)
		elseif player:getStorageValue(Storage.BigfootBurden.QuestLine) < 15 then
			npcHandler:say("Your endurance will be tested here when the time comes. For the moment please continue with the other phases of your recruitment.", cid)
		elseif player:getStorageValue(Storage.BigfootBurden.QuestLine) >= 19 then
			npcHandler:say("You have passed the test. If you consider what huge feet you have to move it's quite impressive.", cid)
		end
	end
	return true
end

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
