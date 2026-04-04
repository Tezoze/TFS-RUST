-- Muhad - Converted from XML to Lua NpcType
-- Original XML: data/npc/Muhad.xml
-- Original Script: data/npc/scripts/Muhad.lua

local npcName = "Muhad"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a muhad")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 146, lookHead = 116, lookBody = 116, lookLegs = 78, lookFeet = 114, lookAddons = 2})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


keywordHandler:addKeyword({'here'}, StdModule.say, {npcHandler = npcHandler, text = 'I am the leader of the true sons of {Daraman}.'})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I am the leader of the true sons of {Daraman}.'})
keywordHandler:addKeyword({'daraman'}, StdModule.say, {npcHandler = npcHandler, text = 'This is our home - the land of the desert.'})
keywordHandler:addKeyword({'ankrahmun'}, StdModule.say, {npcHandler = npcHandler, text = 'We will fight that city until we get back what belongs to us.'})
keywordHandler:addKeyword({'darashia'}, StdModule.say, {npcHandler = npcHandler, text = 'We avoid these places you call cities.'})
keywordHandler:addKeyword({'city'}, StdModule.say, {npcHandler = npcHandler, text = 'I would go crazy living in a cage like that.'})
keywordHandler:addKeyword({'offer'}, StdModule.say, {npcHandler = npcHandler, text = 'We have nothing that would be of value for you.'})
keywordHandler:addKeyword({'undead'}, StdModule.say, {npcHandler = npcHandler, text = 'That is the curse for not following the rules of the desert. No son of the desert has ever come back from the dead.'})
keywordHandler:addKeyword({'daraman'}, StdModule.say, {npcHandler = npcHandler, text = 'We have nothing that would be of value for you.'})

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	local questState = player:getStorageValue(Storage.TibiaTales.AritosTask)

	-- Player asks about Arito
	if msgcontains(msg, "arito") then
		if questState == 1 then
			npcHandler:say({
				'I don\'t know how something like this ever could be possible. He met a girl from Ankrahmun and she must have twisted his head. Arito started to tell stories about the Pharaoh and about Ankrahmun. ...',
				'In the wink of an eye he left us and was never seen again. I think he feared revenge for leaving us - which partially is not without reason. Why are you asking me about him? Did he send you to me?'
			}, cid)
			npcHandler.topic[cid] = 1
		elseif questState >= 2 then
			npcHandler:say('Arito has been acquitted. Tell him he has nothing to fear from us.', cid)
		else
			npcHandler:say('Arito? I have not heard that name in a long time...', cid)
		end
		return true
	end

	-- Player says "yes" - confirming Arito sent them
	if msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say({
				'Ahh, I know that some of my people fear that Arito tells the old secrets of our race and want to see him dead but I don\'t bear him a grudge. I will have to have a serious word with my people. ...',
				'Tell him that he can consider himself as acquitted. He is not the reason for our attacks towards Ankrahmun. Maybe you could help us in this case. Are you willing to do that?'
			}, cid)
			player:setStorageValue(Storage.TibiaTales.AritosTask, 2)
			npcHandler.topic[cid] = 2
			return true
		-- Player agrees to help with Nomads Land Quest (optional continuation)
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say({
				'I appreciate your will to help the sons of the desert. Recently a bunch of thieves have stolen something very valuable from us. It is a secret the true sons kept for aeons and I am not allowed to tell you about it. ...',
				'All we know about the thieves is that they have their hideout somewhere in Ankrahmun. We managed to catch one of them and he told us that there is a pillar in Ankrahmun with a hidden mechanism. ...',
				'If you press the eye of the hawk symbol a secret passage will appear that leads to their hideout. Once inside you have to look for a small casket. ...',
				'Try to sneak in undetectedly and bring back our treasure as soon as you obtain it. May Daraman hold his protective hand over you on your mission. I wish you good luck. ...',
				'One last thing before you leave. Take the path behind me and you will get out of our hideout unharmed.'
			}, cid)
			player:setStorageValue(Storage.TibiaTales.NomadsLand, 1)
			npcHandler.topic[cid] = 0
			return true
		end
	end

	-- Player says "no" - declining to help further
	if msgcontains(msg, "no") then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say('I understand. Please use the back entrance so you don\'t get in trouble with my people.', cid)
			npcHandler.topic[cid] = 0
			return true
		end
	end

	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Be greeted, foreigner under the sun of Darama. What brings you {here}?")
npcHandler:setMessage(MESSAGE_FAREWELL, "May Daraman be with you on your travels.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "May Daraman be with you on your travels.")

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
