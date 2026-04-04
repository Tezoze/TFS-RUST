-- Izsh - Converted from XML to Lua NpcType
-- Original XML: data/npc/Izsh.xml
-- Original Script: data/npc/scripts/Izsh.lua

local npcName = "Izsh"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a izsh")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 338})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	if msgcontains(msg, 'mission') then
		if Player(cid):getStorageValue(Storage.WrathoftheEmperor.Questline) >= 33 then
			npcHandler:say('Oh yez, let me zee ze documentz. Here we go: zree cheztz filled wiz platinum, one houze, a zet of elite armor, and an unending mana cazket. Iz ziz correct?', cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 1 then
		npcHandler:say({
			'Fine, zo let\'z prozeed. You uzed forged documentz to enter our zity, killed zeveral guardz who enjoyed a quite excluzive and expenzive training, deztroyed rare magical devizez in ze pozzezzion of ze emperor. ...',
			'Ze good newz iz, your zree cheztz of platinum should be nearly enough to pay ze finez. Lucky you, ziz could have left you broke. ...',
			'Zere are alzo zertain noble familiez complaining about ze murder of zeveral of zeir beloved onez. ...',
			'I zink I can make a deal wiz ze noblez by zelling zem your property in ze zity. Your prezenze would ruin ze houze prizez zere anyway. ...',
			'Of courze zat will not zuffize to compenzate zeir grief, zo I guezz you\'ll have to part wiz zat elite armor, too. Zadly, prizez for armor are on an all time low right now. ...',
			'But luckily you ztill have zat mana cazket. Well, you had it. Now we have to zell it. ...',
			'But not all iz lozt my blank-zkinned vizitor. According to my calculationz, zere iz ztill a bit left. ...',
			'I zink we can zave you zome gold and zome treazurez, and you can keep one pieze of your elite armor at leazt. ...',
			'You will find your rewardz in one of ze old zupply zellarz. Beware of ze ratz zough. ...',
			'Ze rednezz of your faze and ze zound you make wiz your teez iz obviouzly a zign of gratitude of your zpeziez! I am flattered, but pleaze leave now az I have to attend to zome important buzinezz.'
		}, cid)
		Player(cid):setStorageValue(Storage.WrathoftheEmperor.Questline, 34)
		npcHandler.topic[cid] = 0
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, 'Greetingz zcalelezz being.')

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
