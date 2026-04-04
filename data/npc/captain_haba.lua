-- Captain Haba - Combined NPC for multiple locations
-- Handles: Svargrond (mission) and Ship (travel back)

local npcName = "Captain Haba"
local npcType = Game.createNpcType(npcName)

-- NPC Properties
npcType:name(npcName)
npcType:nameDescription("Captain Haba")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 98})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Per-instance handler storage using position as key
local npcHandlers = {}

local function getHandlerKey(npc)
	local pos = npc:getPosition()
	return string.format("%d_%d_%d", pos.x, pos.y, pos.z)
end

-- Determine NPC type based on position
local function getNpcType(npc)
	local pos = npc:getPosition()
	-- Svargrond ship position (main quest NPC) - around 32345-32360, 31115-31135, z=6
	if pos.z == 6 and pos.x >= 32345 and pos.x <= 32360 and pos.y >= 31115 and pos.y <= 31135 then
		return "svargrond"
	end
	-- Everything else is the ship after hunt (travel back)
	return "ship"
end

local function creatureSayCallback(cid, type, msg)
	local npc = getCurrentNpc()
	if not npc then
		return false
	end
	local handlers = npcHandlers[getHandlerKey(npc)]
	if not handlers then
		return false
	end
	local npcHandler = handlers.npcHandler
	
	if not npcHandler:isFocused(cid) then
		return false
	end
	
	local player = Player(cid)
	local npcLocation = getNpcType(npc)
	
	-- SVARGROND: Sea Serpent Mission
	if npcLocation == "svargrond" then
		if msgcontains(msg, "mission") then
			local storage = player:getStorageValue(Storage.CaptainHaba)
			if storage == -1 or storage == 1 then
				npcHandler:say("Ya wanna join the hunt fo' the sea serpent? Be warned ya may pay with ya life! Are ya in to it?", cid)
				npcHandler.topic[cid] = 1
			elseif storage >= 2 and storage <= 5 then
				npcHandler:say("Ya still need to bring me the {bait} I asked for!", cid)
			elseif storage == 6 then
				npcHandler:say("We have enough bait. Tell me when ya're ready fo' the {hunt}.", cid)
			else
				npcHandler:say("We already went on the hunt together, matey!", cid)
			end
			return true
		elseif msgcontains(msg, "yes") then
			if npcHandler.topic[cid] == 1 then
				npcHandler:say("A'right, we are here to resupply our stock of baits to catch the sea serpent. Your first task is to bring me 5 fish they are easy to catch. When you got them ask me for the bait again.", cid)
				player:setStorageValue(Storage.CaptainHaba, 2)
				npcHandler.topic[cid] = 0
			elseif npcHandler.topic[cid] == 7 then
				npcHandler:say("Let's go fo' a hunt and bring the beast down!", cid)
				player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
				player:teleportTo(Position(31947, 31045, 6), false)
				player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
				npcHandler.topic[cid] = 0
			end
			return true
		elseif msgcontains(msg, "bait") then
			if player:getStorageValue(Storage.CaptainHaba) == 2 then
				if player:removeItem(2667, 5) then
					npcHandler:say("Excellent, now bring me 5 northern pike.", cid)
					player:setStorageValue(Storage.CaptainHaba, 3)
				else
					npcHandler:say("Bring me 5 fish.", cid)
				end
			elseif player:getStorageValue(Storage.CaptainHaba) == 3 then
				if player:removeItem(2669, 5) then
					npcHandler:say("Excellent, now bring me 5 green perch.", cid)
					player:setStorageValue(Storage.CaptainHaba, 4)
				else 
					npcHandler:say("Bring me 5 northern pike.", cid)
				end
			elseif player:getStorageValue(Storage.CaptainHaba) == 4 then
				if player:removeItem(7159, 5) then
					npcHandler:say("Excellent, now bring me 5 rainbow trout.", cid)
					player:setStorageValue(Storage.CaptainHaba, 5)
				else 
					npcHandler:say("Bring me 5 green perch.", cid)
				end
			elseif player:getStorageValue(Storage.CaptainHaba) == 5 then
				if player:removeItem(7158, 5) then
					npcHandler:say("Excellent, that should be enough fish to make the bait. Tell me when ya're ready fo' the hunt.", cid)
					player:setStorageValue(Storage.CaptainHaba, 6)
				else 
					npcHandler:say("Bring me 5 rainbow trout.", cid)
				end
			end
			return true
		elseif msgcontains(msg, "hunt") then
			if player:getStorageValue(Storage.CaptainHaba) == 6 then
				npcHandler:say("A'right, wanna put out to sea?", cid)
				npcHandler.topic[cid] = 7
			end
			return true
		end
	end
	
	-- SHIP: Travel back to Svargrond
	if npcLocation == "ship" then
		if msgcontains(msg, "back") or msgcontains(msg, "svargrond") then
			npcHandler:say("Do you really want go back to my ship in Svargrond?", cid)
			npcHandler.topic[cid] = 10
			return true
		elseif msgcontains(msg, "yes") and npcHandler.topic[cid] == 10 then
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
			player:teleportTo(Position(32348, 31125, 6), false)
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
			npcHandler.topic[cid] = 0
			return true
		elseif msgcontains(msg, "no") and npcHandler.topic[cid] == 10 then
			npcHandler:say("Then not!", cid)
			npcHandler.topic[cid] = 0
			return true
		elseif msgcontains(msg, "job") then
			npcHandler:say("Ye, I command 'is ship!", cid)
			return true
		end
	end
	
	return true
end

local function getHandlers(npc)
	local key = getHandlerKey(npc)
	if not npcHandlers[key] then
		npcHandlers[key] = {
			keywordHandler = KeywordHandler:new(),
			npcHandler = nil
		}
		npcHandlers[key].npcHandler = NpcHandler:new(npcHandlers[key].keywordHandler)
		
		local npcLocation = getNpcType(npc)
		
		if npcLocation == "svargrond" then
			npcHandlers[key].npcHandler:setMessage(MESSAGE_GREET, "Harrr, landlubber wha'd ya want?")
		else
			npcHandlers[key].npcHandler:setMessage(MESSAGE_GREET, "Ye made it! Want to go {back} to Svargrond?")
		end
		
		npcHandlers[key].npcHandler:setMessage(MESSAGE_FAREWELL, "Bye.")
		npcHandlers[key].npcHandler:setMessage(MESSAGE_WALKAWAY, "Bye.")
		npcHandlers[key].npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
		npcHandlers[key].npcHandler:addModule(FocusModule:new())
	end
	return npcHandlers[key]
end

-- NpcType callbacks
npcType:eventType(NPCS_EVENT_APPEAR)
npcType:onAppear(function(npc, creature)
	setCurrentNpc(npc)
	local handlers = getHandlers(npc)
	handlers.npcHandler:onCreatureAppear(creature)
end)

npcType:eventType(NPCS_EVENT_DISAPPEAR)
npcType:onDisappear(function(npc, creature)
	setCurrentNpc(npc)
	local handlers = getHandlers(npc)
	handlers.npcHandler:onCreatureDisappear(creature)
end)

npcType:eventType(NPCS_EVENT_SAY)
npcType:onSay(function(npc, creature, type, message)
	setCurrentNpc(npc)
	local handlers = getHandlers(npc)
	handlers.npcHandler:onCreatureSay(creature, type, message)
end)

npcType:eventType(NPCS_EVENT_THINK)
npcType:onThink(function(npc, interval)
	setCurrentNpc(npc)
	local handlers = getHandlers(npc)
	handlers.npcHandler:onThink()
end)

npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
npcType:onCloseChannel(function(npc, creature)
	setCurrentNpc(npc)
	local handlers = getHandlers(npc)
	handlers.npcHandler:onPlayerCloseChannel(creature)
end)

npcType:register()
