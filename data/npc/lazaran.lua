-- Lazaran - Converted from XML to Lua NpcType
-- Original XML: data/npc/Lazaran.xml
-- Original Script: data/npc/scripts/Lazaran.lua

local npcName = "Lazaran"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a lazaran")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 143, lookHead = 57, lookBody = 57, lookLegs = 57, lookFeet = 57})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "mission") and player:getStorageValue(Storage.TheNewFrontier.Questline) == 9 then
		if npcHandler.topic[cid] == 0 then
			npcHandler:say("Me people wanting peace. No war with others. No war with little men. We few. We weak. Need help. We not wanting make war. No hurt. Say {mission} again.", cid)
			npcHandler.topic[cid] = 10
		elseif npcHandler.topic[cid] == 10 then
			npcHandler:say("You mean you want help us?", cid)
			npcHandler.topic[cid] = 11
		end
	elseif msgcontains(msg, "mission") and player:getStorageValue(Storage.UnnaturalSelection.Questline) < 1 then
		npcHandler:say({
				"Big problem we have! Skull of first leader gone. He ancestor of whole tribe but died long ago in war. We have keep his skull on our sacred place. ...",
				"Then one night, green men came with wolves... and one of wolves took skull and ran off chewing on it! We need back - many wisdom and power is in skull. Maybe they took to north fortress. But can be hard getting in. You try get our holy skull back?"
			}, cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, "mission") and player:getStorageValue(Storage.UnnaturalSelection.Questline) >= 1 then
		if player:getStorageValue(Storage.UnnaturalSelection.Questline) == 1 then
			npcHandler:say("Oh! You found holy skull? In bone pile you found?! Thank Pandor you brought! Me can have it back?", cid)
			npcHandler.topic[cid] = 2
		elseif player:getStorageValue(Storage.UnnaturalSelection.Questline) == 2 then
			npcHandler:say({
				"You brought back skull of first leader. Hero of tribe! But we more missions to do. ...",
				"Me and Ulala talk about land outside. We wanting to see more of land! Land over big water! But us no can leave tribe. No can cross water. Only way is skull of first leader. ...",
				"What holy skull sees, tribe sees. That we believe. Can you bring holy skull over big water and show places?"
			}, cid)
			npcHandler.topic[cid] = 3
		elseif player:getStorageValue(Storage.UnnaturalSelection.Questline) == 3 then
			npcHandler:say("You say you was to where sun is hot and burning? And where trees grow as high as mountain? And where Fasuon cries white tears? Me can't wait to see!! Can have holy skull back?", cid)
			npcHandler.topic[cid] = 4
		elseif player:getStorageValue(Storage.UnnaturalSelection.Questline) == 4 then
			npcHandler:say("We been weak for too long! We prepare for great hunt. But still need many doings! You can help us?", cid)
			npcHandler.topic[cid] = 5
		elseif player:getStorageValue(Storage.UnnaturalSelection.Questline) == 13 then
			npcHandler:say({
				"You well did! Great hunt is under best signs now. We prepare weapons and paint faces and then go! ...",
				"No no, you no need to help us. But can prepare afterparty! Little men sent us stuff some time ago. Was strange water in there. Brown and stinky! But when we tried all tribe became veeeeeeery happy. ...",
				"Now brown water is gone and we sad! Can you bring POT of brown water for party after great hunt? Just bring to me and me trade for shiny treasure."
			}, cid)
			player:setStorageValue(Storage.UnnaturalSelection.Questline, 14)
			player:setStorageValue(Storage.UnnaturalSelection.Mission06, 2) --Questlog, Unnatural Selection Quest "Mission 6: Firewater Burn"
			npcHandler.topic[cid] = 0
		elseif player:getStorageValue(Storage.UnnaturalSelection.Questline) == 14 then
			npcHandler:say("You bring us big pot of strange water from little men?", cid)
			npcHandler.topic[cid] = 6
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("You hero of our tribe if bring back holy skull!", cid)
			player:setStorageValue(Storage.UnnaturalSelection.Questline, 1)
			player:setStorageValue(Storage.UnnaturalSelection.Mission01, 1) --Questlog, Unnatural Selection Quest "Mission 1: Skulled"
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 2 then
			if player:removeItem(11076, 1) then
				npcHandler:say("Me thank you much! All wisdom safe again now.", cid)
				player:setStorageValue(Storage.UnnaturalSelection.Questline, 2)
				player:setStorageValue(Storage.UnnaturalSelection.Mission01, 3) --Questlog, Unnatural Selection Quest "Mission 1: Skulled"
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("You do not have it!", cid)
				npcHandler.topic[cid] = 0
			end
		elseif npcHandler.topic[cid] == 3 then
			npcHandler:say("Here take holy skull. You bring where you think is good. See as much as possible! See where other people live!", cid)
			player:addItem(11076, 1)
			player:setStorageValue(Storage.UnnaturalSelection.Questline, 3)
			player:setStorageValue(Storage.UnnaturalSelection.Mission02, 1) --Questlog, Unnatural Selection Quest "Mission 2: All Around the World"
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 4 then
			if player:getStorageValue(Storage.UnnaturalSelection.Mission02) == 12 and player:getItemCount(11076) >= 1 then
				npcHandler:say("We make big ritual soon and learn much about world outside. Me thank you many times for teaching us world. Very wise and adventurous you are!", cid)
				player:removeItem(11076, 1)
				player:setStorageValue(Storage.UnnaturalSelection.Questline, 4)
				player:setStorageValue(Storage.UnnaturalSelection.Mission02, 13) --Questlog, Unnatural Selection Quest "Mission 2: All Around the World"
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("The skull have not seen all yet!", cid)
				npcHandler.topic[cid] = 0
			end
		elseif npcHandler.topic[cid] == 5 then
			npcHandler:say("We need to calm and make happy gods. Best go to Ulala. She is priest of us and can tell what needs doing.", cid)
			player:setStorageValue(Storage.UnnaturalSelection.Questline, 5)
			player:setStorageValue(Storage.UnnaturalSelection.Mission03, 1) --Questlog, Unnatural Selection Quest "Mission 3: Dance Dance Evolution"
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 6 then
			if player:removeItem(2562, 1, 3) then
				npcHandler:say("We make big ritual soon and learn much about world outside. Me thank you many times for teaching us world. Very wise and adventurous you are!", cid)
				player:addItem(11115, 1)
				player:setStorageValue(Storage.UnnaturalSelection.Questline, 15)
				player:setStorageValue(Storage.UnnaturalSelection.Mission06, 3) --Questlog, Unnatural Selection Quest "Mission 6: Firewater Burn"
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("You do not have the brown strange water!", cid)
				npcHandler.topic[cid] = 0
			end
		elseif npcHandler.topic[cid] == 11 then
			player:setStorageValue(Storage.TheNewFrontier.Questline, 10)
			player:setStorageValue(Storage.TheNewFrontier.Mission03, 2) --Questlog, The New Frontier Quest "Mission 03: Strangers in the Night"
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
