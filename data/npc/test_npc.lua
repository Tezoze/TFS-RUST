-- Test NPC - Converted from XML to Lua NpcType
-- Original XML: data/npc/Test NPC.xml
-- Original Script: data/npc/scripts/TestNPC.lua

local npcName = "Test NPC"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a test npc")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookHead = 95, lookBody = 39, lookLegs = 112, lookFeet = 76, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	npcHandler:setMessage(MESSAGE_GREET, "Hello |PLAYERNAME|! I am a test NPC to verify |PLAYERNAME| replacement works.")
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, "test") then
		npcHandler:say("Your name is |PLAYERNAME|! This should show your actual name.", cid)
		return true
	elseif msgcontains(msg, "keyword") then
		npcHandler:say("Testing keyword approach...", cid)
		return true
	elseif msgcontains(msg, "dialogue") then
		npcHandler:say("Hello there, |PLAYERNAME|! How are you doing today?", cid)
		return true
	end

	return true
end

-- Test keyword approach
local testKeyword = keywordHandler:addKeyword({'testkeyword'}, StdModule.say, {npcHandler = npcHandler, text = 'Your name is |PLAYERNAME|! This uses the keyword handler approach.'})

npcHandler:setCallback(CALLBACK_GREET, greetCallback)
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
