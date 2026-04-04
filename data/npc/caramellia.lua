-- Caramellia - Converted from XML to Lua NpcType
-- Original XML: data/npc/Caramellia.xml
-- Original Script: data/npc/scripts/Caramellia.lua

local npcName = "Caramellia"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a caramellia")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 140, lookHead = 96, lookBody = 34, lookLegs = 34, lookFeet = 34, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'Can I finally have some peace...?' },
	{ text = 'Please leave me alone in my mourning.' }
}

npcHandler:addModule(VoiceModule:new(voices))

keywordHandler:addKeyword({'mourning'}, StdModule.say, {npcHandler = npcHandler, text = "All is lost. With {Winfred} dead, my love has died and I'm only an empty shell without hope or purpose."})
keywordHandler:addKeyword({'port hope'}, StdModule.say, {npcHandler = npcHandler, text = "We put all our hope in this far away colony. Sadly, we never made it there and I will never know what our life would have been like in Port Hope."})
keywordHandler:addKeyword({'druid'}, StdModule.say, {npcHandler = npcHandler, text = "I guess he was the one that put my father up to the whole thing. He spied on Winfred and me and it was certainly him that suggested my {imprisonment} in this tower."})
keywordHandler:addKeyword({'imprisonment'}, StdModule.say, {npcHandler = npcHandler, text = "This forsaken place seems as remote from the rest of the world as my heart is."})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = "Please leave me alone in my mourning."})
keywordHandler:addKeyword({'thais'}, StdModule.say, {npcHandler = npcHandler, text = "The bustling streets of Thais are all but a faint memory to me."})
keywordHandler:addKeyword({'carlin'}, StdModule.say, {npcHandler = npcHandler, text = "Carlin is a lovely and green city as far as I remember."})
keywordHandler:addKeyword({'venore'}, StdModule.say, {npcHandler = npcHandler, text = "My father wanted me to marry a wealthy Venorean. He understood so little about love and the ways of the heart."})
keywordHandler:addKeyword({'ab\'dendriel'}, StdModule.say, {npcHandler = npcHandler, text = "The city of the elves is an exotic wonder."})
keywordHandler:addKeyword({'kazordoon'}, StdModule.say, {npcHandler = npcHandler, text = "The city is like the dwarfs that built it. Stony, never-changing and hard to understand for an outsider."})

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	elseif msgcontains(msg, "winfred") then
		npcHandler:say({
			'He was my one and only true love. He was a mere commoner and so my {father} forbid me to see him ...',
			'We met anyway, we had plans to flee to {Port Hope} and to start a new life there ...',
			'A {druid}, in service of my father, had spied on us. So they brought me here and locked me into this {tower}. The druid cast a spell on the plants at the entrance which kept everyone from entering the tower ...',
			'I could see Winfred from the window but he could not come to me. One day he disappeared. I knew immediately that something horrible had happened to him.'
		}, cid)
	elseif msgcontains(msg, "father") then
		npcHandler:say({
			'He wasn\'t a bad man. He was only misguided by false friends who told him what society expected of him ...',
			'By trying to uphold a respectable image in society, he ruined not only my life but also his own.'
		}, cid)
	elseif msgcontains(msg, "tower") then
		npcHandler:say({
			'Once, this tower has been my prison but after the death of Winfred it has become my refuge from the rest of the world. ...',
			'I welcome the loneliness here because it mirrors the state of my heart.'
		}, cid)
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Hello, |PLAYERNAME|. Please leave me alone in my {mourning}.")
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
