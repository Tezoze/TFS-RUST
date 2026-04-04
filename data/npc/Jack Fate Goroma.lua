-- Jack Fate Goroma - Converted from XML to Lua NpcType
-- Original XML: data/npc/Jack Fate Goroma.xml
-- Original Script: data/npc/scripts/Jack FateGoroma.lua

local npcId = "Jack Fate Goroma"  -- Unique ID for spawn system
local npcDisplayName = "Jack Fate"  -- Name shown in-game
local npcType = Game.createNpcType(npcId)

-- NPC Properties (from XML)
npcType:name(npcDisplayName)
npcType:nameDescription("a jack fate")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 129, lookHead = 19, lookBody = 69, lookLegs = 88, lookFeet = 69})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if (msg) then
		msg = msg:lower()
	end

	if msgcontains(msg, "ship") then
		if player:getStorageValue(Storage.TheShatteredIsles.AccessToGoroma) == 1 then
			npcHandler:say('My ship is finally repaired thanks to your help! Now I can take you back to Liberty Bay anytime. Just say {sail} if you want to travel.', cid)
		elseif (player:getStorageValue(Storage.TheShatteredIsles.Shipwrecked) or -1) == 1 then
			local woodGiven = player:getStorageValue(Storage.TheShatteredIsles.WoodPiecesGiven) or 0
			local woodNeeded = 30 - woodGiven
			npcHandler:say('The ship is still damaged. I need ' .. woodNeeded .. ' more wood pieces to repair it. Have you brought some?', cid)
			npcHandler.topic[cid] = 3
		else
			npcHandler:say('I\'d love to bring you back to Liberty Bay, but as you can see, my ship is ruined. I also hurt my leg and can barely move. Can you help me?', cid)
			npcHandler.topic[cid] = 1
		end
	elseif isInArray({"sail", "passage", "wreck", "liberty bay"}, msg) then
		if player:getStorageValue(Storage.TheShatteredIsles.AccessToGoroma) == 1 then
			npcHandler:say('Do you want to travel back to Liberty Bay?', cid)
			npcHandler.topic[cid] = 4
		elseif (player:getStorageValue(Storage.TheShatteredIsles.Shipwrecked) or -1) == 1 then
			npcHandler:say('My ship is still being repaired. Once it\'s done, I can sail you back to Liberty Bay. Ask about my {ship} to check the progress.', cid)
		else
			npcHandler:say('I can\'t sail you anywhere until my ship is repaired. Ask about my {ship} if you want to help.', cid)
		end
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				"Thank you. Luckily the damage my ship has taken looks more severe than it is, so I will only need a few wooden boards. ...",
				"I saw some lousy trolls running away with some parts of the ship. It might be a good idea to follow them and check if they have some more wood. ...",
				"We will need 30 pieces of wood, no more, no less. Did you understand everything?"
			}, cid)
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say('Good! Please return once you have gathered 30 pieces of wood.', cid)
			player:setStorageValue(Storage.TheShatteredIsles.DefaultStart, 1)
			player:setStorageValue(Storage.TheShatteredIsles.Shipwrecked, 1)
			player:setStorageValue(Storage.TheShatteredIsles.WoodPiecesGiven, 0) -- Initialize wood counter
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 then
			local woodCount = player:getItemCount(5901)
			if woodCount > 0 then
				local woodGiven = player:getStorageValue(Storage.TheShatteredIsles.WoodPiecesGiven) or 0
				local woodNeeded = 30 - woodGiven
				local woodToGive = math.min(woodCount, woodNeeded)

				if woodToGive > 0 then
					player:removeItem(5901, woodToGive)
					local newTotal = woodGiven + woodToGive
					player:setStorageValue(Storage.TheShatteredIsles.WoodPiecesGiven, newTotal)

					if newTotal >= 30 then
						npcHandler:say("Excellent! Now we can leave this godforsaken place. Thank you for your help. Should you ever want to return to this island, ask me for a passage to Goroma.", cid)
						player:setStorageValue(Storage.TheShatteredIsles.Shipwrecked, 2)
						player:setStorageValue(Storage.TheShatteredIsles.AccessToGoroma, 1)
						player:setStorageValue(Storage.TheShatteredIsles.WoodPiecesGiven, 30) -- Ensure it's exactly 30
					else
						local remaining = 30 - newTotal
						npcHandler:say("Thanks for the " .. woodToGive .. " wood piece(s)! I still need " .. remaining .. " more to repair the ship.", cid)
					end
				else
					npcHandler:say("You've already given me enough wood! The ship should be ready now.", cid)
				end
			else
				npcHandler:say("You don't have any wood pieces with you.", cid)
			end
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 4 then
			player:teleportTo(Position(32285, 32892, 6), false)
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
			npcHandler:say('Set the sails!', cid)
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'no') then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say('I understand. Maybe someone else can help me repair my ship.', cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say('I see. Come back when you change your mind.', cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 then
			npcHandler:say('Alright then. Come back when you have more wood.', cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 4 then
			npcHandler:say('As you wish. Let me know if you change your mind.', cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = 'My name is Jack Fate from the Royal Tibia Line.'})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I\'m the captain of this - well, wreck. Argh.'})
keywordHandler:addKeyword({'captain'}, StdModule.say, {npcHandler = npcHandler, text = 'I\'m the captain of this - well, wreck. Argh'})
keywordHandler:addKeyword({'goroma'}, StdModule.say, {npcHandler = npcHandler, text = 'This is where we are... the volcano island Goroma. There are many rumours about this place.'})

npcHandler:setMessage(MESSAGE_GREET, "Hello, Sir |PLAYERNAME|. I hope you can help me with my {ship}.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye then.")

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
