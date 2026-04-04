-- Maris - Converted from XML to Lua NpcType
-- Original XML: data/npc/Maris.xml
-- Original Script: data/npc/scripts/Maris.lua

local npcName = "Maris"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a maris")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 151, lookHead = 78, lookBody = 51, lookLegs = 85, lookFeet = 126})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- Travel
local function addTravelKeyword(keyword, cost, destination)
	local travelKeyword = keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = 'Do you want go to the ' .. keyword:titleCase() .. ' for |TRAVELCOST|?', cost = cost})
		travelKeyword:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, cost = cost, destination = destination})
		travelKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'Alright then!', reset = true})
end

addTravelKeyword('fenrock', 100, Position(32563, 31313, 7))
addTravelKeyword('mistrock', 100, Position(32640, 31439, 7))

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	
	-- Blood Brothers Quest - Garlic Cookie test
	if msgcontains(msg, "garlic cookie") or msgcontains(msg, "cookie") then
		if player:getStorageValue(Storage.BloodBrothers.Mission02) == 1 then
		if player:getItemCount(9116) > 0 then -- Garlic Cookie item ID
			player:removeItem(9116, 1)
				local currentCount = player:getStorageValue(Storage.BloodBrothers.GarlicCookieCount)
				if currentCount == -1 then currentCount = 0 end
				player:setStorageValue(Storage.BloodBrothers.GarlicCookieCount, currentCount + 1)
				player:setStorageValue(Storage.BloodBrothers.MarisSuspect, 1)
				npcHandler:say("A cookie? Well, I suppose... *reluctantly takes a bite* Ugh, too much garlic for my taste, but thanks I guess.", cid)
			else
				npcHandler:say("A cookie? I don't see any cookie with you.", cid)
			end
		else
			npcHandler:say("I'm not really interested in cookies right now.", cid)
		end
	elseif msgcontains(msg, "blood crystal") then
		npcHandler:say("No, thanks.", cid)
		return true
	end

	return true
end

-- Basic
keywordHandler:addKeyword({'offer'}, StdModule.say, {npcHandler = npcHandler, text = 'I can take you to {Fenrock} and {Mistrock}!'})
keywordHandler:addKeyword({'passage'}, StdModule.say, {npcHandler = npcHandler, text = 'I can take you to {Fenrock} and {Mistrock}!'})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I am Maris, Captain of this ship.'})
keywordHandler:addKeyword({'captain'}, StdModule.say, {npcHandler = npcHandler, text = 'I am Maris, Captain of this ship.'})

npcHandler:setMessage(MESSAGE_GREET, "I hope you have a good reason to step near my ship, |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Yeah, bye or whatever.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Bye.")

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
