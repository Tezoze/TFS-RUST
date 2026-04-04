-- Ztiss - Converted from XML to Lua NpcType
-- Original XML: data/npc/Ztiss.xml
-- Original Script: data/npc/scripts/Ztiss.lua

local npcName = "Ztiss"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ztiss")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 340})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, "offer") then
		if player:getStorageValue(Storage.TheNewFrontier.Questline) == 23 then
		npcHandler:say("You are ztill a captive and your life is forfeit. Zere might be a way for you to ezcape if you agree to {work} for my mazter.", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "work") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				"Zere iz a great tournament of ztrengz each decade. It determinez ze granted privilegez for zertain individualz of power for ze comming decade. ...",
				"My mazter wantz to zurprize hiz opponentz by an unexpected move. He will uze warriorz from ze outzide, zomeone zat no one can azzezz. ...",
				"One of ziz warriorz could be you. Or you could ztay here and rot in ze dungeon. Are you interezted in ziz deal?"
			}, cid)
			npcHandler.topic[cid] = 2
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say({
				"You are zmart for a zoftzkin, but before you begin to feel too zmart, you should know zat we will zeal our deal wiz you drinking a ztrong poizon zat will inevitably kill you if you want to trick me and not attend ze tournament. ...",
				"Zo are you ready to drink ziz poizon here?"
			}, cid)
			npcHandler.topic[cid] = 3
		elseif npcHandler.topic[cid] == 3 then
			npcHandler:say({
				"Excellent! Now you may leave ziz area zrough ze teleporter to ze norz. It will bring you to a hidden boat. Ziz boat will take you to ze tournament izle. ...",
				"Zere you'll learn anyzing you need to know about ze great tournament."
			}, cid)
			player:setStorageValue(Storage.TheNewFrontier.Questline, 24)
			player:setStorageValue(Storage.TheNewFrontier.Mission07, 3) --Questlog, The New Frontier Quest "Mission 07: Messengers Of Peace"
			player:setStorageValue(Storage.TheNewFrontier.Mission08, 1) --Questlog, The New Frontier Quest "Mission 08: An Offer You Can't Refuse"
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
