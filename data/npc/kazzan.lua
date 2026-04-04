-- Kazzan - Converted from XML to Lua NpcType
-- Original XML: data/npc/Kazzan.xml
-- Original Script: data/npc/scripts/Kazzan.lua

local npcName = "Kazzan"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a kazzan")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookHead = 95, lookBody = 13, lookLegs = 14, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	npcHandler.topic[cid] = 0
	return true
end


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	-- Pegando a quest
	if msgcontains(msg, "mission") and (player:getStorageValue(Storage.TibiaTales.ToAppeaseTheMightyQuest) or -1) < 1 then
			if (player:getStorageValue(Storage.DjinnWar.Faction.Marid) or -1) < 1 and (player:getStorageValue(Storage.DjinnWar.Faction.Efreet) or -1) < 1 then
			npcHandler:say({
				'Do you know the location of the djinn fortresses in the mountains south of here?'}, cid)
			npcHandler.topic[cid] = 1
		end
	elseif npcHandler.topic[cid] == 1 and msgcontains(msg, "yes") then
			npcHandler:say({
				'Alright. The problem is that I want to know at least one of them on my side. You never know. I don\'t mind if it\'s the evil Efreet or the Marid. ...',
				'Your mission will be to visit one kind of the djinns and bring them a peace-offering. Are you interested in that mission?'
			}, cid)
			npcHandler.topic[cid] = 2
	elseif npcHandler.topic[cid] == 2 and msgcontains(msg, "yes") then
			npcHandler:say({
				'Very good. I hope you are able to convince one of the fractions to stand on our side. If you haven\'t done yet, you should first go and look for old Melchior in Ankrahmun. ...',
				'He knows many things about the djinn race and he may have some hints for you.'
			}, cid)
			if player:getStorageValue(Storage.TibiaTales.DefaultStart) <= 0 then
				player:setStorageValue(Storage.TibiaTales.DefaultStart, 1)
			end
			player:setStorageValue(Storage.TibiaTales.ToAppeaseTheMightyQuest, 1)
		-- Entregando
	elseif player:getStorageValue(Storage.TibiaTales.ToAppeaseTheMightyQuest) == 2 then
		npcHandler:say({
		'Well, I don\'t blame you for that. I am sure you did your best. Now we can just hope that peace remains. Here, take this small gratification for your effort to help and Daraman may bless you!'
		}, cid)
		player:setStorageValue(Storage.TibiaTales.ToAppeaseTheMightyQuest, player:getStorageValue(Storage.TibiaTales.ToAppeaseTheMightyQuest) + 1)
		player:addItem(2152, 20)
	else
		if player:getStorageValue(Storage.WhatAFoolishQuest.Questline) == 35
				and player:getStorageValue(Storage.WhatAFoolishQuest.ScaredKazzan) ~= 1
				and player:getOutfit().lookType == 65 then
			player:setStorageValue(Storage.WhatAFoolishQuest.ScaredKazzan, 1)
			npcHandler:say('WAAAAAHHH!!!', cid)
			return false	
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_ONADDFOCUS, onAddFocus)
npcHandler:setCallback(CALLBACK_ONRELEASEFOCUS, onReleaseFocus)

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
