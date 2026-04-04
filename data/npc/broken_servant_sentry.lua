-- Broken Servant Sentry - Converted from XML to Lua NpcType
-- Original XML: data/npc/Broken Servant Sentry.xml
-- Original Script: data/npc/scripts/Broken Servant Sentry.lua

local npcName = "Broken Servant Sentry"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a broken servant sentry")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 395, lookHead = 58, lookBody = 43, lookLegs = 38, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


function creatureSayCallback(cid, type, msg)
	if(not npcHandler:isFocused(cid)) then
		return false
	end

	if(msgcontains(msg, "slime") or msgcontains(msg, "mould") or msgcontains(msg, "fungus") or msgcontains(msg, "sample")) then
		if(getPlayerStorageValue(cid, 47706) < 1) then
			npcHandler:say("If. You. Bring. Slime. Fungus. Samples. Fro-Fro-Fro-Frrrr*chhhhchrk*From. Other. Tower. You. Must. Be. The. Master. Are. You. There. Master?", cid)
			npcHandler.topic[cid] = 1
		elseif(getPlayerStorageValue(cid, 47706) == 1) then
			npcHandler:say("If. You. Bring. Slime. Fungus. Samples. Fro-Fro-Fro-Frrrr*chhhhchrk*From. Other. Tower. You. Must. Be. The. Master. Are. You. There. Master?", cid)
			npcHandler.topic[cid] = 3
		end

	elseif(msgcontains(msg, "cap") or msgcontains(msg, "mage")) then
		if(getPlayerItemCount(cid, 13756) >= 1 and getPlayerStorageValue(cid, 47706) == 2) and getPlayerStorageValue(cid, 47707) < 1 then
			selfSay("Yo-Yo-Your*chhhrk*. Cap. Is. Slimed. I. Can. Clean. It. *chhhhrrrkchrk* ...", cid)
			npcHandler:say("Here. You. Are. *chhhrrrrkchrk*", cid)
			doPlayerRemoveItem(cid, 13756, 1)
			setPlayerStorageValue(cid, 47707, 1)
			doPlayerAddOutfit(cid, 432, 1)
			doPlayerAddOutfit(cid, 433, 1)
			npcHandler.topic[cid] = 0
		elseif(getPlayerStorageValue(cid, 47707) == 1) then
			selfSay("You already have this outfit!", cid)
			npcHandler.topic[cid] = 0
		end


	elseif(msgcontains(msg, "staff") or msgcontains(msg, "spike")) then
		if(getPlayerItemCount(cid, 13940) >= 1 and getPlayerStorageValue(cid, 47706) == 2) and getPlayerStorageValue(cid, 47708) < 1 then
			npcHandler:say({"Yo-Yo-Your*chhhrk*. Cap. Is. Slimed. I. Can. Clean. It. *chhhhrrrkchrk* ...",
				"Here. You. Are. *chhhrrrrkchrk*"}, cid, 0, 1, 4000)
			doPlayerRemoveItem(cid, 13940, 1)
			setPlayerStorageValue(cid, 47708, 1)
			doPlayerAddOutfit(cid, 432, 2)
			doPlayerAddOutfit(cid, 433, 2)
			npcHandler.topic[cid] = 0
		elseif(getPlayerStorageValue(cid, 47708) == 1) then
			selfSay("You already have this outfit!", cid)
			npcHandler.topic[cid] = 0
		end

	elseif(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 1) then
				npcHandler:say("I. Greet. You. Ma-Ma-Ma-ster! Did. You. Bring. Mo-Mo-Mo-M*chhhhrrrk*ore. Samples. For. Me. To-To-To. Analyse-lyse-lyse?", cid)
				npcHandler.topic[cid] = 2
		elseif(npcHandler.topic[cid] == 2) then
				npcHandler:say("Thank. I. Will. Start. Analysing. No-No-No-No*chhrrrk*Now.", cid)
				setPlayerStorageValue(cid, 47706, 1)
				setPlayerStorageValue(cid, Storage.OutfitQuest.DefaultStart, 1) --this for default start of Outfit and Addon Quests
				npcHandler.topic[cid] = 0
		elseif(npcHandler.topic[cid] == 3) then
				npcHandler:say("I. Greet. You. Ma-Ma-Ma-ster! Did. You. Bring. Mo-Mo-Mo-M*chhhhrrrk*ore. Samples. For. Me. To-To-To. Analyse-lyse-lyse?", cid)
				npcHandler.topic[cid] = 4
		elseif(npcHandler.topic[cid] == 4) and getPlayerItemCount(cid, 13758) >= 20 then
				npcHandler:say({"Please. Wait. I. Can. Not. Han-Han-Han*chhhhhrrrchrk*Handle. *chhhhrchrk* This. Is. Enough. Material. *chrrrchhrk* ...",
				"I. Have-ve-ve-veee*chrrrck*. Also. Cleaned. Your. Clothes. Master. It. Is. No-No-No*chhrrrrk*Now. Free. Of. Sample. Stains."}, cid, 0, 1, 4000)
				doPlayerRemoveItem(cid, 13758, 20)
				setPlayerStorageValue(cid, 47706, 2)
				doPlayerAddOutfit(cid, 432, 0)
				doPlayerAddOutfit(cid, 433, 0)
				npcHandler.topic[cid] = 0
			else
				selfSay("You do not have all the required items.", cid)
				npcHandler.topic[cid] = 0
			end
	return true

end
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
end


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
