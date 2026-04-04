-- A Wandering Soul - Converted from XML to Lua NpcType
-- Original XML: data/npc/A Wandering Soul.xml
-- Original Script: data/npc/scripts/A Wandering Soul.lua

local npcName = "A Wandering Soul"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a a wandering soul")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(1500)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookBody = 11, lookLegs = 10, lookFeet = 28})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	-- Blood Brothers Quest - Blood Crystal charging
	if msgcontains(msg, "blood crystal") then
		if player:getStorageValue(Storage.BloodBrothers.BloodCrystal.Quest) == 1 then
			if player:getItemCount(9369) > 0 then -- Has Blood Crystal
				if player:getStorageValue(Storage.BloodBrothers.BloodCrystal.Charged) ~= 1 then
					-- Charge the crystal - remove uncharged and give charged
					player:removeItem(9369, 1) -- Remove uncharged crystal
					player:addItem(9141, 1) -- Give charged crystal
					player:setStorageValue(Storage.BloodBrothers.BloodCrystal.Charged, 1)
					npcHandler:say("What the...? I don't know anything about that... still... the pain felt a little less intense for a second... relieving... but will never be gone.", cid)
					player:getPosition():sendMagicEffect(CONST_ME_MAGIC_RED)
				else
					npcHandler:say("Your blood crystal is already charged with my energy.", cid)
				end
			else
				npcHandler:say("You don't have a blood crystal to charge. Find one first!", cid)
			end
		else
			npcHandler:say("What are you talking about? I have lost everything... my life, my love, my soul...", cid)
		end
		return true
	end

	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Ohhhh... who... who are you? What do you want from me?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Farewell... may you find peace...")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Ohhhh...")

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
