-- Eclesius - Converted from XML to Lua NpcType
-- Original XML: data/npc/Eclesius.xml
-- Original Script: data/npc/scripts/Eclesius.lua

local npcName = "Eclesius"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a eclesius")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookBody = 91, lookLegs = 12, lookFeet = 95, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = "I'm looking for a new assistant!" },
	{ text = "Err, what was it again that I wanted...?" },
	{ text = "Do come in! Mind the step of the magical door, though." },
	{ text = "I'm so sorry... I promise it won't happen again. Problem is, I can't remember where I made the error..." },
	{ text = "Actually, I STILL prefer inexperienced assistants. They're easier to keep an eye on and don't tend to backstab you." },
	{ text = "So much to do, so much to do... uh... where should I start?" }
}


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	
	
	--Cuadno responde Mission
	if msgcontains(msg, 'mission') then	
		if player:getStorageValue(45215) == -1 then
			npcHandler:say({
				'Well, now that you mention it, there is in fact something you could do for me apart from your usual tasks. It\'s all because of that fool! ...',
				'I can\'t remember who it was, but someone ruined my favourite hat which used to complete my favourite outfit. It\'s totally dented! Also there are some suspicious stains on it. It is out of the question for me to wear it in that state!! ...',
				'So I guess I need a new hat. And while we\'re at it, I want it to be kind of stylish. Also manly. Not pink! And, uh, not too heavy. But since you\'re my assistant, I\'ll leave all that up to you. Hehe. ...',
				'Unfortunately I\'m not good at sewing, so you might need to find a tailor. I heard they have a large warehouse for clothing in {Venore}. Maybe you can find someone there who could help you with the hat. Will you take on this mission?'
			}, cid)
			npcHandler.topic[cid] = 1  
		end
		
		if player:getStorageValue(45215) == 2 then
			npcHandler:say({
				'Oh wow. A hat, you say? You\'ve actually brought me a new hat?! How did you know I wanted one? That\'s almost sweet of you <sniff>. Can I see it?'
			}, cid)
			npcHandler.topic[cid] = 2 
		end
		
		
	end
	
	
	--Cuadno responde YES
	if msgcontains(msg, 'yes') then	
		if npcHandler.topic[cid] == 1  then
			npcHandler:say({
				'<claps hands> How splendid! ... What was it again? Uhm... I can\'t remember right now, but I feel happy anyway. Just surprise me with whatever you have planned. Hehe.'
			}, cid)
			player:setStorageValue(45215, 1) 
		end
		
		if npcHandler.topic[cid] == 2  then			
			if player:removeItem(10046, 1) then
				npcHandler:say({
				'<claps hands> How splendid! ... What was it again? Uhm... I can\'t remember right now, but I feel happy anyway. Just surprise me with whatever you have planned. Hehe.'
				}, cid)
				player:setStorageValue(45215, 3) 
				player:addExperience(10000)
				doPlayerAddItem(player,2152,50)
			else
				npcHandler:say("His is not my hat =(", cid)
			end
			
		end
		
		
		
		
	end

	return true
end




npcHandler:addModule(VoiceModule:new(voices))
npcHandler:setMessage(MESSAGE_GREET, "Who are you? What do you want? You seem too experienced to become my assistant. Please leave.")
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
