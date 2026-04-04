-- A Fading Memory - Converted from XML to Lua NpcType
-- Original XML: data/npc/A Fading Memory.xml
-- Original Script: data/npc/scripts/A Fading Memory.lua

local npcName = "A Fading Memory"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a a fading memory")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 138})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	
	-- Blood Brothers Quest Mission 9 - Kala's story for defeating Marziel
	if msgcontains(msg, "kala") then
		if player:getStorageValue(Storage.BloodBrothers.Mission09) == 1 then
			npcHandler:say("... yes! That's my name... how come you know that?", cid)
		else
			npcHandler:say("...", cid)
		end
	
	elseif msgcontains(msg, "diary") then
		if player:getStorageValue(Storage.BloodBrothers.Mission09) == 1 then
			npcHandler:say("... you... read Marziel's diary and know our story...?", cid)
		else
			npcHandler:say("... I don't understand...", cid)
		end
	
	elseif msgcontains(msg, "vampire") then
		if player:getStorageValue(Storage.BloodBrothers.Mission09) == 1 then
			npcHandler:say("... so there is nothing I could have done...? He's a vampire now... what can we do...", cid)
		else
			npcHandler:say("... dark creatures...", cid)
		end
	
	elseif msgcontains(msg, "free soul") then
		if player:getStorageValue(Storage.BloodBrothers.Mission09) == 1 then
			npcHandler:say("... he can't be freed from his curse that easily... he must be awaken first...", cid)
		else
			npcHandler:say("... souls are trapped...", cid)
		end
	
	elseif msgcontains(msg, "awaken") then
		if player:getStorageValue(Storage.BloodBrothers.Mission09) == 1 then
			npcHandler:say("... to awake him... I don't know but... he once truly loved me... maybe there is still something left... somewhere... here... take this from me....and thank you for listening...", cid)
			-- Give player a special item needed to awaken Marziel (this could be used in the boss fight)
			player:addItem(7461, 1) -- Love Letter or similar item
			npcHandler:disappear()
		else
			npcHandler:say("... awakening... yes... but how...", cid)
		end
	end
	
	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Ohh...")
npcHandler:setMessage(MESSAGE_FAREWELL, "...")
npcHandler:setMessage(MESSAGE_WALKAWAY, "...")

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
