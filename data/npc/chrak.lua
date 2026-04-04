-- Chrak - Converted from XML to Lua NpcType
-- Original XML: data/npc/Chrak.xml
-- Original Script: data/npc/scripts/Chrak.lua

local npcName = "Chrak"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a chrak")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 115})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, "battle") then
		if player:getStorageValue(Storage.TheNewFrontier.Questline) == 24 then
			npcHandler:say({
				"Zo you want to enter ze arena, you know ze rulez and zat zere will be no ozer option zan deaz or victory? ...",
				"My mazter wantz to zurprize hiz opponentz by an unexpected move. He will uze warriorz from ze outzide, zomeone zat no one can azzezz. ...",
				"One of ziz warriorz could be you. Or you could ztay here and rot in ze dungeon. Are you interezted in ziz deal?"
			}, cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.TheNewFrontier.Questline) == 26 then
			npcHandler:say({
				"You have done ze impozzible and beaten ze champion. Your mazter will be pleazed. Hereby I cleanze ze poizon from your body. You are now allowed to leave. ...",
				"For now ze mazter will zee zat you and your alliez are zpared of ze wraz of ze dragon emperor az you are unimportant for hiz goalz. ...",
				"You may crawl back to your alliez and warn zem of ze gloriouz might of ze dragon emperor and hiz minionz."
			}, cid)
			player:setStorageValue(Storage.TheNewFrontier.Questline, 27)
			player:setStorageValue(Storage.TheNewFrontier.Mission09, 3) --Questlog, The New Frontier Quest "Mission 09: Mortal Combat"
			player:unregisterEvent("NewFrontierTirecz")
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("Asss you wishzz.", cid)
			player:setStorageValue(Storage.TheNewFrontier.Questline, 25)
			player:setStorageValue(Storage.TheNewFrontier.Mission08, 2) --Questlog, The New Frontier Quest "Mission 08: An Offer You Can't Refuse"
			player:setStorageValue(Storage.TheNewFrontier.Mission09, 1) --Questlog, The New Frontier Quest "Mission 09: Mortal Combat"
			player:registerEvent("NewFrontierTirecz") -- Dynamic registration for Tirecz events
			npcHandler.topic[cid] = 0
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
