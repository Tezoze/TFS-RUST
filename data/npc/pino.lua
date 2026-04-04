-- Pino - Converted from XML to Lua NpcType
-- Original XML: data/npc/Pino.xml
-- Original Script: data/npc/scripts/Pino.lua

local npcName = "Pino"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a pino")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 115, lookLegs = 67, lookFeet = 114})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Feel the wind in your hair during one of my carpet rides!'} }
npcHandler:addModule(VoiceModule:new(voices))

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, 'fly') then
			npcHandler:say('The different places we travel to are: {darashia}, {svargrond}, {femor hills}, {edron}, {Kazordoon}', cid)
			return true
	end
	return true
end

-- Travel
local function addTravelKeyword(keyword, text, cost, destination)
	local travelKeyword = keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = 'Do you seek a ride to ' .. text .. ' for |TRAVELCOST|?', cost = cost, discount = 'postman'})
		travelKeyword:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, text = 'Hold on!', cost = cost, discount = 'postman', destination = destination})
		travelKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'You shouldn\'t miss the experience.', reset = true})
end

-- Special handling for Farmine
local farmineKeyword = keywordHandler:addKeyword({'farmine'}, function(cid, type, msg, matches, node)
    local player = Player(cid)
    if not player then
        npcHandler:say("Something went wrong. Please try again.", cid)
        return true
    end
    local accessValue = player:getStorageValue(Storage.TheNewFrontier.Mission10) or -1
    if accessValue < 1 then
        npcHandler:say('Never heard about a place like this.', cid)
        return true
    else
        npcHandler:say('Do you seek a ride to Farmine for 60 gold pieces?', cid)
        npcHandler.topic[cid] = 1 -- Set topic to allow child keywords
        return true
    end
end, {npcHandler = npcHandler})

-- Child keywords for Farmine travel
farmineKeyword:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, text = 'Hold on!', cost = 60, discount = 'postman', destination = Position(32983, 31539, 1)})
farmineKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'You shouldn\'t miss the experience.', reset = true})

addTravelKeyword('darashia', 'Darashia on Darama', 60, Position(33270, 32441, 6))
addTravelKeyword('kazordoon', 'Kazordoon', 80, Position(32588, 31941, 0))
addTravelKeyword('femor hills', 'the Femor Hills', 60, Position(32536, 31837, 4))
addTravelKeyword('svargrond', 'Svargrond', 40, Position(32253, 31097, 4))
addTravelKeyword('edron', 'Edron', 60, Position(33193, 31784, 3))
addTravelKeyword('hills', 'the Femor Hills', 60, Position(32536, 31837, 4))

npcHandler:setMessage(MESSAGE_GREET, "Greetings, traveller |PLAYERNAME|. Where do you want me to {fly} you?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye!")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye!")
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
