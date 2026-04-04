-- Sinatuki - Converted from XML to Lua NpcType
-- Original XML: data/npc/Sinatuki.xml
-- Original Script: data/npc/scripts/Sinatuki.lua

local npcName = "Sinatuki"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a sinatuki")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 260})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


keywordHandler:addKeyword({'Chuqua'}, StdModule.say, {npcHandler = npcHandler, text = "Chuqua jamjam!! Tiyopa Sinatuki?"})

local fishsID = {7158,7159}

function creatureSayCallback(cid, type, msg)

local player = Player(cid)

	if(not npcHandler:isFocused(cid)) then
		return false
	end
		
	if msgcontains(msg, 'Nupi') then
	if player:getStorageValue(Storage.BarbarianTest.Questline) >= 3 and player:getStorageValue(Storage.TheIceIslands.Questline) >=5 then
		for i=1, #fishsID do 
			if player:getItemCount(fishsID[i]) >= 100 then		
				player:removeItem(fishsID[i], 100) 							  
				player:addItem(7290, 5)
				npcHandler:say("Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you and gives you five glimmering crystals>", cid)
			break
			elseif player:getItemCount(fishsID[i]) >= 99 then
				player:removeItem(fishsID[i], 99) 							  
				player:addItem(7290, 5)
				npcHandler:say("Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you>", cid)
			break
			else 
				npcHandler:say("Kisavuta! <giggles>", cid)
			end
		end	
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
