-- Mr. West - Converted from XML to Lua NpcType
-- Original XML: data/npc/Mr. West.xml
-- Original Script: data/npc/scripts/Mr West.lua

local npcName = "Mr. West"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a mr. west")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 151, lookHead = 58, lookBody = 25, lookLegs = 29, lookFeet = 114})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	local player = Player(cid)
	if(player:getStorageValue(Storage.InServiceofYalahar.MrWestDoor) == 1) then
		npcHandler:setMessage(MESSAGE_GREET, "Wh .. What? How did you get here? Where are all the guards? You .. you could have killed me but yet you chose to talk? What a relief! ... So what brings you here my friend, if I might call you like that? ")
	elseif(player:getStorageValue(Storage.InServiceofYalahar.MrWestDoor) == 2) then
		npcHandler:setMessage(MESSAGE_GREET, "Murderer! But .. I give in, you won! ... Dictate me your conditions but please, I beg you, spare my life. What do you want?")
	end
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if(msgcontains(msg, "mission")) then
		if(player:getStorageValue(Storage.InServiceofYalahar.Questline) == 24) then
			if(player:getStorageValue(Storage.InServiceofYalahar.MrWestDoor) == 1) then
				npcHandler:say("Indeed, I can see the benefits of a mutual agreement. I will later read the details and send a letter to your superior. ", cid)
				player:setStorageValue(Storage.InServiceofYalahar.Questline, 25)
				player:setStorageValue(Storage.InServiceofYalahar.Mission04, 3) -- StorageValue for Questlog "Mission 04: Good to be Kingpin"
				player:setStorageValue(Storage.InServiceofYalahar.MrWestStatus, 1)
				npcHandler.topic[cid] = 0
			elseif(player:getStorageValue(Storage.InServiceofYalahar.MrWestDoor) == 2) then
				npcHandler:say("Yes, for the sake of my life I'll accept those terms. I know when I have lost. Tell your master I will comply with his orders. ", cid)
				player:setStorageValue(Storage.InServiceofYalahar.Questline, 25)
				player:setStorageValue(Storage.InServiceofYalahar.Mission04, 4) -- StorageValue for Questlog "Mission 04: Good to be Kingpin"
				player:setStorageValue(Storage.InServiceofYalahar.MrWestStatus, 2)
				npcHandler.topic[cid] = 0
			end
		end
	end
	return true
end

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
