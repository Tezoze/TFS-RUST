-- Uzon - Converted from XML to Lua NpcType
-- Original XML: data/npc/Uzon.xml
-- Original Script: data/npc/scripts/Uzon.lua

local npcName = "Uzon"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
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
local function addTravelKeyword(keyword, text, cost, destination, condition, action)
	if condition then
		keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = 'Never heard about a place like this.'}, condition)
	end

	local travelKeyword = keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = text, cost = cost, discount = 'postman'})
		travelKeyword:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, text = 'Hold on!', cost = cost, discount = 'postman', destination = destination}, nil, action)
		travelKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'You shouldn\'t miss the experience.', reset = true})
end

local travelKeyword = keywordHandler:addKeyword({'eclipse'}, function(cid, type, msg, matches, node)
	local player = Player(cid)
	if not player then
		npcHandler:say("Something went wrong. Please try again.", cid)
		return true
	end
	local m = player:getStorageValue(Storage.TheInquisition.Mission02) or -1
	if m ~= 1 and m ~= 2 then
		npcHandler:say('Never heard about a place like this.', cid)
		return true
	else
		npcHandler:say('Oh no, so the time has come? Do you really want me to fly you to this unholy place?', cid)
		npcHandler.topic[cid] = 1 -- Set topic to allow child keywords
		return true
	end
end, {npcHandler = npcHandler})

-- Child keywords for travel
travelKeyword:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, text = 'Hold on!', cost = 110, discount = 'postman', destination = Position(32659, 31915, 0)})
travelKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'You shouldn\'t miss the experience.', reset = true})
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
addTravelKeyword('edron', 'Do you seek a ride to Edron for |TRAVELCOST|?', 60, Position(33193, 31783, 3), nil, function(player) if player:getStorageValue(Storage.postman.Mission01) == 2 then player:setStorageValue(Storage.postman.Mission01, 3) end end)
addTravelKeyword('darashia', 'Do you seek a ride to Darashia on Darama for |TRAVELCOST|?', 60, Position(33270, 32441, 6))
addTravelKeyword('svargrond', 'Do you seek a ride to Svargrond for |TRAVELCOST|?', 60, Position(32253, 31097, 4))
addTravelKeyword('kazordoon', 'Do you seek a ride to Kazordoon for |TRAVELCOST|?', 60, Position(32588, 31942, 0))

-- Basic
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = "I am known as Uzon Ibn Kalith."})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = "I am a licensed Darashian carpet pilot. I can bring you to {Darashia}, {Kazordoon}, {Svargrond} or {Edron}."})
keywordHandler:addKeyword({'caliph'}, StdModule.say, {npcHandler = npcHandler, text = "The caliph welcomes travellers to his land."})
keywordHandler:addKeyword({'kazzan'}, StdModule.say, {npcHandler = npcHandler, text = "The caliph welcomes travellers to his land."})
keywordHandler:addKeyword({'daraman'}, StdModule.say, {npcHandler = npcHandler, text = "Oh, there is so much to tell about Daraman. You better travel to Darama to learn about his teachings."})
keywordHandler:addKeyword({'ferumbras'}, StdModule.say, {npcHandler = npcHandler, text = "I would never transport this one."})
keywordHandler:addKeyword({'drefia'}, StdModule.say, {npcHandler = npcHandler, text = "So you heard about haunted Drefia? Many adventures travel there to test their skills against the undead: vampires, mummies, and ghosts."})
keywordHandler:addKeyword({'excalibug'}, StdModule.say, {npcHandler = npcHandler, text = "Some people claim it is hidden somewhere under the endless sands of the devourer desert in Darama."})
keywordHandler:addKeyword({'thais'}, StdModule.say, {npcHandler = npcHandler, text = "Thais is noisy and overcrowded. That's why I like Darashia more."})
keywordHandler:addKeyword({'tibia'}, StdModule.say, {npcHandler = npcHandler, text = "I have seen almost every place on the continent."})
keywordHandler:addKeyword({'continent'}, StdModule.say, {npcHandler = npcHandler, text = "I could retell the tales of my travels for hours. Sadly another flight is scheduled soon."})
keywordHandler:addKeyword({'carlin'}, StdModule.say, {npcHandler = npcHandler, text = "Just another Thais but with women to lead them."})
keywordHandler:addKeyword({'flying'}, StdModule.say, {npcHandler = npcHandler, text = "You can buy flying carpets only in Darashia."})
keywordHandler:addKeyword({'new'}, StdModule.say, {npcHandler = npcHandler, text = "I heard too many news to recall them all."})
keywordHandler:addKeyword({'rumors'}, StdModule.say, {npcHandler = npcHandler, text = "I heard too many news to recall them all."})
keywordHandler:addKeyword({'passage'}, StdModule.say, {npcHandler = npcHandler, text = "I can fly you to {Darashia} on Darama, {Kazordoon}, {Svargrond} or {Edron} if you like. Where do you want to go?"})
keywordHandler:addKeyword({'transport'}, StdModule.say, {npcHandler = npcHandler, text = "I can fly you to {Darashia} on Darama, {Kazordoon}, {Svargrond} or {Edron} if you like. Where do you want to go?"})
keywordHandler:addKeyword({'ride'}, StdModule.say, {npcHandler = npcHandler, text = "I can fly you to {Darashia} on Darama, {Kazordoon}, {Svargrond} or {Edron} if you like. Where do you want to go?"})
keywordHandler:addKeyword({'trip'}, StdModule.say, {npcHandler = npcHandler, text = "I can fly you to {Darashia} on Darama, {Kazordoon}, {Svargrond} or {Edron} if you like. Where do you want to go?"})

npcHandler:setMessage(MESSAGE_GREET, "Daraman's blessings, traveller |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Daraman's blessings")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Daraman's blessings")

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
