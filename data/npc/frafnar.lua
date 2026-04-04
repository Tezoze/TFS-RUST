-- Frafnar - Converted from XML to Lua NpcType
-- Original XML: data/npc/Frafnar.xml
-- Original Script: data/npc/scripts/Frafnar.lua

local npcName = "Frafnar"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a frafnar")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 160, lookHead = 58, lookBody = 119, lookLegs = 81, lookFeet = 114})
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
		if player:getStorageValue(Storage.hiddenCityOfBeregar.SweetAsChocolateCake) < 1 then
			npcHandler:say("There is indeed something you could do for me. You must know, I'm in love with Bolfana. I'm sure she'd have a beer with me if I got her a chocolate cake. Problem is that I can't leave this door as I'm on duty. Would you be so kind and help me?", cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(Storage.hiddenCityOfBeregar.SweetAsChocolateCake) == 2 then
			npcHandler:say("So did you tell her that the cake came from me?", cid)
			npcHandler.topic[cid] = 2
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			player:setStorageValue(Storage.hiddenCityOfBeregar.SweetAsChocolateCake, 1)
			player:setStorageValue(Storage.hiddenCityOfBeregar.DefaultStart, 1)
			npcHandler:say("Great! She works in the tavern of Beregar. It's situated in the western part of the city. Bring her a chocolate cake and tell her that it was me who sent it.", cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 2 then
			player:setStorageValue(Storage.hiddenCityOfBeregar.SweetAsChocolateCake, 3)
			player:setStorageValue(Storage.hiddenCityOfBeregar.DoorWestMine, 1)
		npcHandler:say("Great! That's my breakthrough. Now she can't refuse to go out with me. I grant you access to the western part of the mine.", cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_WALKAWAY, "See you my friend.")
npcHandler:setMessage(MESSAGE_FAREWELL, "See you my friend.")
npcHandler:setMessage(MESSAGE_GREET, "Don't you see that I'm trying to write a poem? <sighs> So what's the matter?")
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
