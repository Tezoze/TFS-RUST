-- Pythius the Rotten - Converted from XML to Lua NpcType
-- Original XML: data/npc/Pythius the Rotten.xml
-- Original Script: data/npc/scripts/Pythius The Rotten.lua

local npcName = "Pythius the Rotten"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a pythius the rotten")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 231})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local treasureKeyword = keywordHandler:addKeyword({"treasure"}, StdModule.say, {npcHandler = npcHandler, text = "LIKE MY TREASURE? WANNA PICK SOMETHING OUT OF IT?"})
	treasureKeyword:addChildKeyword({"yes"}, StdModule.say, {npcHandler = npcHandler, text = "ALRIGHT. BUT FIRST OF ALL I WANT YOU TO BRING ME SOMETHING IN EXCHANGE. SURPRISE ME....AND IF I LIKE IT, YOU MAY GET WHAT YOU DESERVE.", reset = true})
	treasureKeyword:addChildKeyword({"no"}, StdModule.say, {npcHandler = npcHandler, text = "HAVE YOU SEEN THESE LEGENDARY ITEMS BACK THERE? WHO COULD REFUSE THE CHANCE OF OBTAINING ONE?!? SO WHAT IS YOUR ANSWER?"})

local offerKeyword = keywordHandler:addKeyword({"offer"}, StdModule.say, {npcHandler = npcHandler, text = "I GRANT YOU ACCESS TO THE DUNGEON IN THE NORTH. YOU'LL FIND SOME OF MY LIVING BROTHERS THERE....BUT.....EVERY TIME YOU WANT TO ENTER YOU HAVE TO GIVE ME SOMETHING PRECIOUS. ALRIGHT?"}, function(player) return player:getLevel() > 99 end)
	local mugKeyword = offerKeyword:addChildKeyword({"yes"}, StdModule.say, {npcHandler = npcHandler, text = "AS YOU WISH. WHAT DO YOU HAVE TO OFFER?"})
		mugKeyword:addChildKeyword({"golden mug"}, StdModule.say, {npcHandler = npcHandler, text = "I LIKE THAT AND GRANT YOU ACCESS TO THE DUNGEON IN THE NORTH FOR THE NEXT FEW MINUTES. COME BACK ANYTIME AND BRING ME MORE TREASURES.", reset = true},
			function(player) return player:getItemCount(2033) > 0 end,
			function(player)
				player:removeItem(2033, 1)
				player:setStorageValue(Storage.hiddenCityOfBeregar.PythiusTheRotten, os.time() + 180)
			end
		)
		mugKeyword:addChildKeyword({"golden mug"}, StdModule.say, {npcHandler = npcHandler, text = "THIS IS NOT WORTH BEING PART OF MY TREASURE! BRING ME SOMETHING ELSE.", reset = true})
		mugKeyword:addChildKeyword({""}, StdModule.say, {npcHandler = npcHandler, text = "THIS IS NOT WORTH BEING PART OF MY TREASURE! BRING ME SOMETHING ELSE", reset = true})
	offerKeyword:addChildKeyword({""}, StdModule.say, {npcHandler = npcHandler, text = "TELL ME IF YOU CHANGE YOUR MIND. MY TREASURE THIRSTS FOR GOLD.", reset = true})
keywordHandler:addKeyword({"offer"}, StdModule.say, {npcHandler = npcHandler, text = "YOU LITTLE MAGGOT. COME BACK TO ME WHEN YOU CAN HANDLE A FIGHT AGAINST MY KIND."})

-- Basic keywords
keywordHandler:addKeyword({"awaited"}, StdModule.say, {npcHandler = npcHandler, text = "I HAVE A MISSION FOR YOU BUT YOU NEED TO DIE FIRST AND RETURN AS AN {UNDEAD} CREATURE. COME BACK TO ME WHEN YOU ACHIEVED THIS GOAL."})
keywordHandler:addKeyword({"exchange"}, StdModule.say, {npcHandler = npcHandler, text = "EVERYTHING YOU CARRY WITH YOU CAN ALSO BE FOUND IN MY {TREASURE}. BRING ME SOMETHING I DON'T OWN!!!"})
keywordHandler:addKeyword({"mission"}, StdModule.say, {npcHandler = npcHandler, text = "I HAVE A MISSION FOR YOU BUT YOU NEED TO DIE FIRST AND RETURN AS AN {UNDEAD} CREATURE. COME BACK TO ME WHEN YOU ACHIEVED THIS GOAL."})
keywordHandler:addKeyword({"undead"}, StdModule.say, {npcHandler = npcHandler, text = "BOON AND BANE. I HAVE CHOSEN THIS LIFE VOLUNTARILLY AND I NEVER REGRET IT. MY {TREASURE} IS GROWING BIGGER EACH DAY."})

npcHandler:setMessage(MESSAGE_GREET, "I {AWAITED} YOU!")
npcHandler:setMessage(MESSAGE_FAREWELL, "COME BACK ANYTIME AND BRING ME TREASURES.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "COME BACK ANYTIME AND BRING ME TREASURES.")

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
