-- Ocelus - Converted from XML to Lua NpcType
-- Original XML: data/npc/Ocelus.xml
-- Original Script: data/npc/scripts/Ocelus.lua

local npcName = "Ocelus"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ocelus")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 80})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "eleonore") then
		if player:getStorageValue(Storage.TheShatteredIsles.APoemForTheMermaid) == 2 and player:getStorageValue(Storage.TheShatteredIsles.ADjinnInLove) < 1 then
			npcHandler:say("I heard the birds sing about her beauty. But how could a human rival the enchanting beauty of a {mermaid}?", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "mermaid") or msgcontains(msg, "marina") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				"Oh yes, I noticed that lovely mermaid. From afar of course. I would not dare to step into the eyes of such a lovely creature. ...",
				"... I guess I am quite shy. Oh my, if I were not blue, I would turn red now. If there would be someone to arrange a {date} with her."
			}, cid)
			npcHandler.topic[cid] = 2
		elseif player:getStorageValue(Storage.TheShatteredIsles.ADjinnInLove) == 2 then
			npcHandler:say("Oh my. Its not easy to impress a mermaid I guess. Please get me a {love poem}. I think elves are the greatest poets so their city seems like a good place to look for one.", cid)
			player:setStorageValue(Storage.TheShatteredIsles.ADjinnInLove, 3)
		end
	elseif msgcontains(msg, "date") then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say("Will you ask the mermaid Marina if she would date me?", cid)
			npcHandler.topic[cid] = 3
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 3 then
			npcHandler:say("Thank you. How ironic, a human granting a djinn a wish.", cid)
			npcHandler.topic[cid] = 0
			player:setStorageValue(Storage.TheShatteredIsles.ADjinnInLove, 1)
		elseif npcHandler.topic[cid] == 4 then
			if player:removeItem(8189, 1) then
				npcHandler:say("Excellent. Here, with this little spell I enable you to recite the poem like a true elven poet. Now go and ask her for a date again.", cid)
				player:setStorageValue(Storage.TheShatteredIsles.ADjinnInLove, 4)
				player:setStorageValue(Storage.TheShatteredIsles.APoemForTheMermaid, 3)
				npcHandler.topic[cid] = 0
			else
				npcHandler.topic[cid] = 0
				npcHandler:say("You don't have it...", cid)
			end
		end
	elseif msgcontains(msg, "love poem") then
		if player:getStorageValue(Storage.TheShatteredIsles.ADjinnInLove) == 3 then
			npcHandler:say("Did you get a love poem from Ab'Dendriel?", cid)
			npcHandler.topic[cid] = 4
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Greetings, dear visitor |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Oh well.")

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
