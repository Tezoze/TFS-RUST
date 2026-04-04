-- Tomruk The Ruddy - Converted from XML to Lua NpcType
-- Original XML: data/npc/Tomruk The Ruddy.xml
-- Original Script: data/npc/scripts/Tomruk The Ruddy.lua

local npcName = "Tomruk The Ruddy"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a tomruk the ruddy")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 553})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'Interesting reaction. Bloodcurdling. Must inspect further.' },
	{ text = 'This is really sanguine!' },
	{ text = 'Hmm... the conductors are too dry to transmit energy.' },
	{ text = 'Ah, fresh blood. My favourite.' }
}

npcHandler:addModule(VoiceModule:new(voices))

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if(msgcontains(msg, 'scroll') or msgcontains(msg, 'mission')) and player:getStorageValue(Storage.GravediggerOfDrefia.Mission35) == 1 and player:getStorageValue(Storage.GravediggerOfDrefia.Mission36) < 1 then
		npcHandler:say({
			"So someone sent you after a scroll, eh? A stroll for a scroll! <chuckles> Sounds like an old-fashioned necromancer thing. ...",
			"Well, this piece here is a rather fascinating thing - see those strange blood stains? - ...",
			"My predecessors have had it for quite a while without finding out more - I was hoping to investigate, but there's always so much to do! ...",
			"Ah, so you want it, too? Well, do me a favour: fetch two samples and assist in my experiment, in exchange for the scroll piece. Yes?"
		}, cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 1 and player:getStorageValue(Storage.GravediggerOfDrefia.Mission35) == 1 then
		npcHandler:say({
			"Sanguine! I need two different blood samples - The first one from the necromancer's pure blood chamber. ... ",
			"I was barred from the premises. For my research! Shameful! I'm a martyr to the cause - oh, the second sample you said? ...",
			"The second sample you must retrieve from the sacrificial chamber in the ancient vampire crypts, first floor, far west. ...",
			"Take these two sterilised vials, one for each blood basin. Oh, I wish I could go myself! Come back when you have filled both vials."
		}, cid)
		player:setStorageValue(Storage.GravediggerOfDrefia.Mission36, 1)
		player:addItem(21417, 2)
		npcHandler.topic[cid] = 0
	elseif(msgcontains(msg, 'scroll') or msgcontains(msg, 'mission') or msgcontains(msg, 'blood')) and player:getStorageValue(Storage.GravediggerOfDrefia.Mission37) == 1 and player:getStorageValue(Storage.GravediggerOfDrefia.Mission38) < 1 then
		npcHandler:say("Hello hello! Did you bring those blood samples?", cid)
		npcHandler.topic[cid] = 2
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 2 and player:getStorageValue(Storage.GravediggerOfDrefia.Mission37) == 1 then
		if player:getItemCount(21419) >= 1 and player:getItemCount(21418) >= 1 then
			npcHandler:say({
				"Now, let me see... yes... yes... very good. Let me add THIS ..... swill it... there. Sanguine! ...",
				"We're not finished yet. Take this tainted blood vial ...",
				"Dab some drops from it on to the four blood pagodas in the inner circle here. Then pull the lightning lever over there."
			}, cid)
			player:setStorageValue(Storage.GravediggerOfDrefia.Mission38, 1)
			player:removeItem(21418, 1)
			player:removeItem(21419, 1)
			player:addItem(21449, 1)
			npcHandler.topic[cid] = 0
		else
			npcHandler:say("You haven't got any blood.", cid)
			npcHandler.topic[cid] = 0
		end
	elseif(msgcontains(msg, 'scroll') or msgcontains(msg, 'mission')) and player:getStorageValue(Storage.GravediggerOfDrefia.Mission40) == 1 and player:getStorageValue(Storage.GravediggerOfDrefia.Mission41) < 1 then
		npcHandler:say("Hello hello! Did Hello hello! Well now, painted all those blood pagodas properly?", cid)
		npcHandler.topic[cid] = 3
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 3 and player:getStorageValue(Storage.GravediggerOfDrefia.Mission40) == 1 then
		npcHandler:say({
			"Sanguine! Did you see those sparks! We definitely had some energy transfer! Well done! Now, for your reward. ...",
			"Err... I would awfully like to know more about the scroll ...",
			"Would you settle for a heartfelt handshake instead - oh, you wouldn't? Well, er, okay ...",
			"Give me a minute or two to retrieve it. Ask me for the {scroll} or the {mission}"
		}, cid)
		player:setStorageValue(Storage.GravediggerOfDrefia.Mission41, 1)
		npcHandler.topic[cid] = 0
	elseif(msgcontains(msg, 'scroll') or msgcontains(msg, 'mission')) and player:getStorageValue(Storage.GravediggerOfDrefia.Mission41) == 1 and player:getStorageValue(Storage.GravediggerOfDrefia.Mission42) < 1 then
		npcHandler:say("Hello hell- oh, you've come for the scroll, haven't you?", cid)
		npcHandler.topic[cid] = 4
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 4 and player:getStorageValue(Storage.GravediggerOfDrefia.Mission41) == 1 then
		npcHandler:say("My heart bleeds to part from it. Here. Extend your hand - I'll just retrieve some blood from in exchange - HOLD STILL.", cid)
		player:setStorageValue(Storage.GravediggerOfDrefia.Mission42, 1)
		player:addItem(21250, 1)
		npcHandler.topic[cid] = 0
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
