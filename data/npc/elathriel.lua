-- Elathriel - Converted from XML to Lua NpcType
-- Original XML: data/npc/Elathriel.xml
-- Original Script: data/npc/scripts/Elathriel.lua

local npcName = "Elathriel"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a elathriel")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 64})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


keywordHandler:addKeyword({'business'}, StdModule.say, {npcHandler = npcHandler, text = 'I am the leader of the Kuridai and the Az\'irel of Ab\'Dendriel. Humans would call it {sheriff}, executioner, or avenger.'})
keywordHandler:addKeyword({'sheriff'}, StdModule.say, {npcHandler = npcHandler, text = 'Sometimes people get imprisoned for some time. True criminals will be cast out and for comitting the worst crimes offenders are thrown into the hellgate.'})
keywordHandler:addKeyword({'executioner'}, StdModule.say, {npcHandler = npcHandler, text = 'Sometimes people get imprisoned for some time. True criminals will be cast out and for comitting the worst crimes offenders are thrown into the hellgate.'})
keywordHandler:addKeyword({'avenger'}, StdModule.say, {npcHandler = npcHandler, text = 'Sometimes people get imprisoned for some time. True criminals will be cast out and for comitting the worst crimes offenders are thrown into the hellgate.'})
keywordHandler:addKeyword({'hellgate'}, StdModule.say, {npcHandler = npcHandler, text = 'It was here among other structures, like the depot tower, before our people came here. It\'s secured by a sealed door.'})
keywordHandler:addKeyword({'sealed'}, StdModule.say, {npcHandler = npcHandler, text = 'For safety we keep the door to the hellgate locked all times. I have the {keys} to open it when needed.'})
keywordHandler:addKeyword({'door'}, StdModule.say, {npcHandler = npcHandler, text = 'For safety we keep the door to the hellgate locked all times. I have the {keys} to open it when needed.'})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = 'Not that I like to talk to you, but I am Elathriel Shadowslayer.'})
keywordHandler:addKeyword({'army'}, StdModule.say, {npcHandler = npcHandler, text = 'It\'s one of the more useful concepts we can learn from the other races.'})
keywordHandler:addKeyword({'king'}, StdModule.say, {npcHandler = npcHandler, text = 'It\'s hard for some of my people to grasp the true concept of a strong leader.'})
keywordHandler:addKeyword({'magic'}, StdModule.say, {npcHandler = npcHandler, text = 'I mastered some battle spells.'})
keywordHandler:addKeyword({'time'}, StdModule.say, {npcHandler = npcHandler, text = 'I couldn\'t care less.'})
keywordHandler:addKeyword({'eloise'}, StdModule.say, {npcHandler = npcHandler, text = 'A human weakling, not much more.'})
keywordHandler:addKeyword({'tibianus'}, StdModule.say, {npcHandler = npcHandler, text = 'A human weakling, not much more.'})
keywordHandler:addKeyword({'druid'}, StdModule.say, {npcHandler = npcHandler, text = 'Druidic magic is too peaceful for my taste.'})
keywordHandler:addKeyword({'sorcerer'}, StdModule.say, {npcHandler = npcHandler, text = 'I have seen human sorcerers doing some impressive things ... before they died.'})
keywordHandler:addKeyword({'dwarfs'}, StdModule.say, {npcHandler = npcHandler, text = 'We might use the shelter earth and hills provide us, but their obsession for metal is a waste of time.'})
keywordHandler:addKeyword({'trolls'}, StdModule.say, {npcHandler = npcHandler, text = 'Like all inferior races, they can be at least of use for something good. The other castes are just jealous of us making use of them.'})
keywordHandler:addKeyword({'excalibug'}, StdModule.say, {npcHandler = npcHandler, text = 'I still doubt it exists.'})
keywordHandler:addKeyword({'ferumbras'}, StdModule.say, {npcHandler = npcHandler, text = 'Even if he\'d walk through the town above, the other castes would not see the necessity to follow OUR way.'})
keywordHandler:addKeyword({'elves'}, StdModule.say, {npcHandler = npcHandler, text = 'My people are divided in castes in these times, until they comprehend that only the way of the Kuridai can save us all.'})
keywordHandler:addKeyword({'cenath'}, StdModule.say, {npcHandler = npcHandler, text = 'Arrogant bastards, but they wield quite powerful magic.'})
keywordHandler:addKeyword({'teshial'}, StdModule.say, {npcHandler = npcHandler, text = 'Dreamers are of no practical use. I don\'t mourn their demise.'})
keywordHandler:addKeyword({'deraisim'}, StdModule.say, {npcHandler = npcHandler, text = 'Confused cowards. For all their skill, they still tend to hide and run. What a waste.'})
keywordHandler:addKeyword({'kuridai'}, StdModule.say, {npcHandler = npcHandler, text = 'We are the heart of the elven society. We forge, we build, and we don\'t allow our people to be pushed around.'})
keywordHandler:addKeyword({'carlin'}, StdModule.say, {npcHandler = npcHandler, text = 'We watch this city and the actions of its inhabitants closely.'})
keywordHandler:addKeyword({'venore'}, StdModule.say, {npcHandler = npcHandler, text = 'The merchants of venore provide us with some usefull goods. Still I an convinced that they get more out of our bargain then we do.'})
keywordHandler:addKeyword({'thais'}, StdModule.say, {npcHandler = npcHandler, text = 'The Thaian kingdom and we share some enemies, so it\'s only logical to cooperate in a few areas.'})
keywordHandler:addKeyword({'carlin'}, StdModule.say, {npcHandler = npcHandler, text = ''})
keywordHandler:addKeyword({'offer'}, StdModule.say, {npcHandler = npcHandler, text = 'I offer hardly anything except for knowledge of {spells}.'})
keywordHandler:addKeyword({'buy'}, StdModule.say, {npcHandler = npcHandler, text = 'I offer hardly anything except for knowledge of {spells}.'})
keywordHandler:addKeyword({'sell'}, StdModule.say, {npcHandler = npcHandler, text = 'I offer hardly anything except for knowledge of {spells}.'})

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	if msgcontains(msg, 'key') then
		npcHandler:say('If you are that curious, do you want to buy a key for 5000 gold? Don\'t blame me if you get sucked in.', cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			local player = Player(cid)
			if player:removeMoneyNpc(5000) then
				npcHandler:say('Here it is.', cid)
				local key = player:addItem(2089, 1)
				if key then
					key:setActionId(3012)
				end
			else
				npcHandler:say('Come back when you have enough money.', cid)
			end
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'no') then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say('Believe me, it\'s better for you that way.', cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, 'Be greeted |PLAYERNAME|. What is your {business} near the {hellgate}?')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Asha Thrazi, stranger.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Asha Thrazi, stranger.')

local focusModule = FocusModule:new()
focusModule:addGreetMessage({'hi', 'hello', 'ashari'})
focusModule:addFarewellMessage({'bye', 'farewell', 'asgha thrazi'})
npcHandler:addModule(focusModule)


	keywordHandler:addSpellKeyword({'avalanche'}, {npcHandler = npcHandler, spellName = 'Avalanche', price = 1200, level = 30, vocation ={2}})
	keywordHandler:addSpellKeyword({'explosion'}, {npcHandler = npcHandler, spellName = 'Explosion', price = 1800, level = 31, vocation ={2}})
	keywordHandler:addSpellKeyword({'heavy','magic','missile'}, {npcHandler = npcHandler, spellName = 'Heavy Magic Missile', price = 1500, level = 25, vocation ={2}})
	keywordHandler:addSpellKeyword({'ice','wave'}, {npcHandler = npcHandler, spellName = 'Ice Wave', price = 850, level = 18, vocation ={2}})
	keywordHandler:addSpellKeyword({'light','magic','missile'}, {npcHandler = npcHandler, spellName = 'Light Magic Missile', price = 500, level = 15, vocation ={2}})
	keywordHandler:addSpellKeyword({'stalagmite'}, {npcHandler = npcHandler, spellName = 'Stalagmite', price = 1400, level = 24, vocation ={2}})
	keywordHandler:addSpellKeyword({'terra','wave'}, {npcHandler = npcHandler, spellName = 'Terra Wave', price = 2500, level = 38, vocation ={2}})
	keywordHandler:addKeyword({'attack', 'spells'}, StdModule.say, {npcHandler = npcHandler, text = "In this category I have '{Ice Wave}' and '{Terra Wave}'."})
	keywordHandler:addKeyword({'support', 'spells'}, StdModule.say, {npcHandler = npcHandler, text = "In this category I have '{Avalanche}', '{Explosion}', '{Heavy Magic Missile}', '{Light Magic Missile}' and '{Stalagmite}'."})
	keywordHandler:addKeyword({'spells'}, StdModule.say, {npcHandler = npcHandler, text = 'I can teach you {Attack spells} and {Support spells}.'})


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

npcType:register()
