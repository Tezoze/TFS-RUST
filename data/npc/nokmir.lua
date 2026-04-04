-- Nokmir - Converted from XML to Lua NpcType
-- Original XML: data/npc/Nokmir.xml
-- Original Script: data/npc/scripts/Nokmir.lua

local npcName = "Nokmir"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a nokmir")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 160, lookHead = 57, lookBody = 87, lookLegs = 59, lookFeet = 114})
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
		if player:getStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll) < 1 then
			npcHandler:say("I don't see how you could help me. I'm in deep, deep trouble. I'm accused of having stolen a {ring} from Rerun, but I haven't.", cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll) == 5 then
			player:setStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll, 6)
			player:setStorageValue(Storage.hiddenCityOfBeregar.DoorNorthMine, 1)
			npcHandler:say("WHAT?! I can't believe it. You saved my life... well, at least one week of it 'cause that would have been the time I had to spend in jail. If you want to, you can pass the door now and take a look at the northern mines. Have fun!", cid)
		end
	elseif msgcontains(msg, "ring") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				"He said he still had it after work. On that evening, {Grombur}, {Rerun} and me opened a cask of beer in one of the mine tunnels. We had a fun evening there. ...",
				"On the next day, the guards brought me to emperor {Rehal}, and Rerun was there, too. He said I had stolen his ring. I'd never steal, you have to believe me."
			}, cid)
			npcHandler.topic[cid] = 2
		end
	elseif msgcontains(msg, "grombur") then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say("Maybe Grombur knows more than me. The thing is he won't talk to me, and he will surely not accuse his best friend as a liar. What a dilemma!", cid)
			npcHandler.topic[cid] = 3
		end
	elseif msgcontains(msg, "rerun") then
		if npcHandler.topic[cid] == 3 then
			npcHandler:say("He's a miner in the southern wing. Maybe he has lost the ring there... but even if I find the ring, no one will believe me. Someone should talk to Grombur. He's Rerun's best friend.", cid)
			npcHandler.topic[cid] = 4
		end
	elseif msgcontains(msg, "rehal") then
		if npcHandler.topic[cid] == 4 then
			player:setStorageValue(Storage.hiddenCityOfBeregar.DefaultStart, 1)
			player:setStorageValue(Storage.hiddenCityOfBeregar.JusticeForAll, 1)
			npcHandler:say("He's a good emperor but I doubt he is wise enough to see the truth behind that false accusation against me. If just someone would find out the truth about that whole mess.", cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_WALKAWAY, "See you my friend.")
npcHandler:setMessage(MESSAGE_FAREWELL, "See you my friend.")
npcHandler:setMessage(MESSAGE_GREET, "You are....kind of tall! Hello.")
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
