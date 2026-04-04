-- Gnomaticus - Converted from XML to Lua NpcType
-- Original XML: data/npc/Gnomaticus.xml
-- Original Script: data/npc/scripts/Gnomaticus.lua

local npcName = "Gnomaticus"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a gnomaticus")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 493, lookHead = 1, lookBody = 86, lookLegs = 1, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, "again") then
		player:setStorageValue(Storage.BigfootBurden.QuestLine, 19)
	end

	if msgcontains(msg, "shooting") then
		if player:getStorageValue(Storage.BigfootBurden.QuestLine) == 11 then
			npcHandler:say({
				"To the left you see our shooting range. Grab a cannon and shoot at the targets. You need five hits to succeed. ...",
				"Shoot at the villain targets that will pop up. DON'T shoot innocent civilians since this will reset your score and you have to start all over. Report to me afterwards."
			}, cid)
			player:setStorageValue(Storage.BigfootBurden.QuestLine, 13) -- tirar do questlog
			player:setStorageValue(Storage.BigfootBurden.Shooting, 0)
		elseif player:getStorageValue(Storage.BigfootBurden.QuestLine) == 13 then
			npcHandler:say("Shoot at the villain targets that will pop up. DON'T shoot innocent civilians since this will reset your score and you have to start all over. {Report} to me afterwards.", cid)
		end
	elseif msgcontains(msg, "report") then
		if player:getStorageValue(Storage.BigfootBurden.QuestLine) == 14 then
			npcHandler:say("You are showing some promise! Now continue with the recruitment and talk to Gnomewart to the south for your endurance test!", cid)
			player:setStorageValue(Storage.BigfootBurden.Shooting, player:getStorageValue(Storage.BigfootBurden.Shooting) + 1)
			player:setStorageValue(Storage.BigfootBurden.QuestLine, 15)
		elseif player:getStorageValue(Storage.BigfootBurden.QuestLine) == 13 then
			npcHandler:say("Sorry you are not done yet.", cid)
		elseif player:getStorageValue(Storage.BigfootBurden.QuestLine) <= 12 then
			npcHandler:say("You have nothing to report at all.", cid)
		end
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
