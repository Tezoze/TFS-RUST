-- UzonE - Converted from XML to Lua NpcType
-- Original XML: data/npc/UzonE.xml
-- Original Script: data/npc/scripts/UzonE.lua

local npcId = "UzonE"  -- Unique ID for spawn system
local npcDisplayName = "Uzon"  -- Name shown in-game
local npcType = Game.createNpcType(npcId)

-- NPC Properties (from XML)
npcType:name(npcDisplayName)
npcType:nameDescription("a uzon")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookHead = 95, lookBody = 4, lookLegs = 17, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Feel the wind in your hair during one of my carpet rides!'} }
npcHandler:addModule(VoiceModule:new(voices))

keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = "I am known as Uzon Ibn Kalith."})
--keywordHandler:addKeyword({'passage'}, StdModule.say, {npcHandler = npcHandler, text = "You'll have to leave this unholy place first!"})
--keywordHandler:addKeyword({'transport'}, StdModule.say, {npcHandler = npcHandler, text = "You'll have to leave this unholy place first!"})
keywordHandler:addKeyword({'ride'}, StdModule.say, {npcHandler = npcHandler, text = "You'll have to leave this unholy place first!"})
keywordHandler:addKeyword({'trip'}, StdModule.say, {npcHandler = npcHandler, text = "You'll have to leave this unholy place first!"})

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	if (msg) then
		msg = msg:lower()
	end

	if isInArray({"back", "leave", "passage", "transport", "eclipse"}, msg) then
		npcHandler:say('Can we finally leave this cursed place?', cid)
		npcHandler.topic[cid] = 1
	elseif(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 1) then
			local player, destination = Player(cid), Position(32535, 31837, 4)
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
			player:teleportTo(destination)
			destination:sendMagicEffect(CONST_ME_TELEPORT)
			npcHandler:say('So be it!', cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, "Daraman's blessings, traveller |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Daraman's blessings")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Daraman's blessings")


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
