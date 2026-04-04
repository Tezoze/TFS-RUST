-- A Dead Bureaucrat1 - Converted from XML to Lua NpcType
-- Original XML: data/npc/A Dead Bureaucrat1.xml
-- Original Script: data/npc/scripts/A Dead Bureaucrat1.lua

local npcName = "A Dead Bureaucrat1"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a a dead bureaucrat")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 33})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'Now where did I put that form?' },
	{ text = 'Hail Pumin. Yes, hail.' }
}

npcHandler:addModule(VoiceModule:new(voices))

local config = {
	[1] = "wand",
	[2] = "rod",
	[3] = "bow",
	[4] = "sword"
}

local function greetCallback(cid)
	npcHandler:setMessage(MESSAGE_GREET, "Hello " .. (Player(cid):getSex() == PLAYERSEX_FEMALE and "beautiful lady" or "handsome gentleman") .. ", welcome to the atrium of Pumin's Domain. We require some information from you before we can let you pass. Where do you want to go?")
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	local vocationId = player:getVocation():getBase():getId()

	if msgcontains(msg, "pumin") then
		if npcHandler.topic[cid] == 0 and player:getStorageValue(Storage.PitsOfInferno.ThronePumin) < 1 then
			npcHandler:say("Sure, where else. Everyone likes to meet my master, he is a great demon, isn't he? Your name is ...?", cid)
			npcHandler.topic[cid] = 1
		elseif npcHandler.topic[cid] == 3 then
			player:setStorageValue(Storage.PitsOfInferno.ThronePumin, 1)
			npcHandler:say("How very interesting. I need to tell that to my master immediately. Please go to my colleagues and ask for Form 356. You will need it in order to proceed.", cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, player:getName()) then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("Alright |PLAYERNAME|. Vocation?", cid)
			npcHandler.topic[cid] = 2
		end
	elseif msgcontains(msg, Vocation(vocationId):getName()) then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say("Huhu, please don't hurt me with your " .. config[vocationId] .. "! Reason of your visit?", cid)
			npcHandler.topic[cid] = 3
		end
	elseif msgcontains(msg, "411") then
		if player:getStorageValue(Storage.PitsOfInferno.ThronePumin) == 3 then
			npcHandler:say("Form 411? You need Form 287 to get that! Do you have it?", cid)
			npcHandler.topic[cid] = 4
		elseif player:getStorageValue(Storage.PitsOfInferno.ThronePumin) == 5 then
			npcHandler:say("Form 411? You need Form 287 to get that! Do you have it?", cid)
			npcHandler.topic[cid] = 5
		end
	elseif msgcontains(msg, "no") then
		if npcHandler.topic[cid] == 4 then
			player:setStorageValue(Storage.PitsOfInferno.ThronePumin, 4)
			npcHandler:say("Oh, what a pity. Go see one of my colleagues. I give you the permission to get Form 287. Bye!", cid)
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 5 then
			player:setStorageValue(Storage.PitsOfInferno.ThronePumin, 6)
			npcHandler:say("Great. Here you are. Form 411. Come back anytime you want to talk. Bye.", cid)
		end
	elseif msgcontains(msg, "356") then
		if player:getStorageValue(Storage.PitsOfInferno.ThronePumin) == 8 then
			player:setStorageValue(Storage.PitsOfInferno.ThronePumin, 9)
			npcHandler:say("INCREDIBLE, you did it!! Have fun at Pumin's Domain!", cid)
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye and don't forget me!")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye and don't forget me!")

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
