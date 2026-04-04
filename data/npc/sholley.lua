-- Sholley - Converted from XML to Lua NpcType
-- Original XML: data/npc/Sholley.xml
-- Original Script: data/npc/scripts/Sholley.lua

local npcName = "Sholley"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a sholley")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 140, lookHead = 79, lookBody = 86, lookLegs = 12, lookFeet = 92, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



function creatureSayCallback(cid, type, msg)
	if(not(npcHandler:isFocused(cid))) then
		return false
	end


	if(msgcontains(msg, "friend")) then
		if(getPlayerStorageValue(cid, 10050) == 12) then
			selfSay("So you have proven yourself a true friend of our city. It's hard to believe but I think your words only give substance to suspicions my heart had harboured since quite a while. ...", cid)
			selfSay("So Harsin is probably not the person he appeared to be. Actually I haven't heard from him for quite a while. He was resident in the local bed and breakfast hotel. You should be able to find him there or at least to learn about his whereabouts.", cid)
            setPlayerStorageValue(cid, 10050, 13)
		   setPlayerStorageValue(cid, 30051, 1)
		   setPlayerStorageValue(cid, 30052, 0)
		end
	elseif(msgcontains(msg, "quandon")) then
		if(getPlayerStorageValue(cid, 10050) == 15) then
			selfSay("A transporter dead? This is more then alarming. It seems Harsin is up to something and whatever it is, it's nothing good at all. But not all is lost. A local medium, Barnabas, has truly the gift to speak to the dead. ...", cid)
			selfSay("I'll mark his home on your map. He should be able to get the information you need to locate Harsin.", cid)
            setPlayerStorageValue(cid, 10050, 16)
		   setPlayerStorageValue(cid, 30055, 0)
		end
		else
		selfSay("Already clicked the body on the house Roswitha ?", cid)

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
