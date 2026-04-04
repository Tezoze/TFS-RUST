-- Cerdras - Converted from XML to Lua NpcType
-- Original XML: data/npc/Cerdras.xml
-- Original Script: data/npc/scripts/Cerdras.lua

local npcName = "Cerdras"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a cerdras")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 144, lookHead = 20, lookBody = 96, lookLegs = 41, lookFeet = 22, lookAddons = 2})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = "I'm merely a humble druid like so many others here. I may not be the most talented of healers, but I am gifted with a special atunement to the elements."})
keywordHandler:addKeyword({'nature'}, StdModule.say, {npcHandler = npcHandler, text = "For me, nature is the harmony of the elements. This harmony can be disturbed by certain events, but nature always finds its way back to harmony in the end."})

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	elseif msgcontains(msg, "elements") then
		npcHandler:say({
			'How can I explain my connection to the elements so that you can understand it? Hmmm, it is like a faint melody, a song, that is always there. ...',
			'I hear that melody shifting in time with the shifts in the elements. With so many years of listening, I have learned to interpret these shifts and so come to a deeper understanding of the elements. ...',
			'It was a natural step for me to become responsible for researching elemental lore. I try to learn as much as I can and share it with my fellow druids. ...',
			'Unfortunately, much of my understanding is instinctive, and our language just doesn\'t contain the right words for me to express the things I feel adequately.'
		}, cid)
	elseif msgcontains(msg, "song") then
		npcHandler:say({
			'It is hard to explain. Of course, it\'s not a real song as you would understand it. I don\'t hear it with my ears, but rather, I feel it deep inside of me. ...',
			'Calling it a song or melody is the best I can do to describe it to those who don\'t share this kind of perception. ...',
			'It also helps me to express and understand something for which our language has no appropriate expression. ...',
			'You know, we are so dependent on words that we can\'t think about concepts when we don\'t have words for them. ...',
			'I sometimes think words have become just as much of a hindrance as a help. ...',
			'Perhaps we would fare better if only we forgot words and dealt purely in feelings. Then perhaps all of us could hear the wonderful melody of nature.'
		}, cid)
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, "Greetings my friend!")
npcHandler:setMessage(MESSAGE_FAREWELL, "May Crunor bless and guide you, |PLAYERNAME|.")


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
