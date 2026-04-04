-- Cledwyn - Converted from XML to Lua NpcType
-- Original XML: data/npc/Cledwyn.xml
-- Original Script: data/npc/scripts/Cledwyn.lua

local npcName = "Cledwyn"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a cledwyn")
npcType:health(150)
npcType:maxHealth(150)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 114, lookLegs = 76, lookFeet = 94, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)
local talkState = {}


local items = {
     ['magic'] = {
          ['ice'] = 25193,
          ['fire'] = 25190,
          ['earth'] = 25191,
          ['energy'] = 25192,
     },
     ['magic level'] = {
          ['ice'] = 25193,
          ['fire'] = 25190,
          ['earth'] = 25191,
          ['energy'] = 25192,
     },
     ['sword'] = {
          ['ice'] = 25183,
          ['fire'] = 25174,
          ['earth'] = 25177,
          ['energy'] = 25180,
     },
     ['axe'] = {
          ['ice'] = 25184,
          ['fire'] = 25175,
          ['earth'] = 25178,
          ['energy'] = 25181,
     },
     ['club'] = {
          ['ice'] = 25185,
          ['fire'] = 25176,
          ['earth'] = 25179,
          ['energy'] = 25182,
     },
     ['distance'] = {
          ['ice'] = 25189,
          ['fire'] = 25186,
          ['earth'] = 25187,
          ['energy'] = 25188,
     },
}

local skillChoice = {}

local function greetCallback(cid)
    skillChoice[cid] = nil
    return true
end


function creatureSayCallback(cid, type, msg)
     if not npcHandler:isFocused(cid) then
               return false
     end

     local player = Player(cid)
     if not player then
          return false
     end

     if msgcontains(msg, 'tokens') then
          npcHandler:say("If you have any {silver} tokens with you, let's have a look! Maybe I can offer you something in exchange.", cid)
     elseif msgcontains(msg, 'information') then
          npcHandler:say("With pleasure. <bows> I trade {tokens}. There are several ways to obtain the tokens I am interested in - killing certain bosses, for example. In exchange for a certain amount of tokens, I can offer you some first-class items.", cid)
     elseif msgcontains(msg, 'talk') then
          npcHandler:say({"Why, certainly! I'm always up for some small talk. ...",
                         "The weather continues just fine here, don't you think? Just the day for a little walk around the town! ...",
                         "Actually, I haven't been around much yet, but I'm looking forward to exploring the city once I've finished trading {tokens}."}, cid)
     elseif msgcontains(msg, 'silver') then
          npcHandler:say({"Here's the deal, " .. player:getName() .. ". For 100 of your silver tokens, I can offer you some first-class torso armor. These armors provide a solid boost to your main attack skill, as well as ...",
                         "some elemental protection of your choice! So, which skill type are you most interested in: {sword}, {axe}, {club}, {distance} or {magic level}?"}, cid)
          npcHandler.topic[cid] = 1
     elseif isInArray({'sword', 'axe', 'club', 'distance', 'magic', 'magic level'}, msg:lower()) then
          if npcHandler.topic[cid] == 1 then
               local skill = msg:lower()
               if not items[skill] then
                    return false
               else
                    skillChoice[cid] = skill
                    npcHandler:say("Ah, very good. Now choose an element against which this armor will provide additional protection: {fire}, {earth}, {energy} or {ice}.", cid)
                    npcHandler.topic[cid] = 2
               end
          end
     elseif isInArray({'fire', 'earth', 'energy', 'ice'}, msg:lower()) then
          if npcHandler.topic[cid] == 2 then
               local element = msg:lower()
               if not items[skillChoice[cid]][element] then
                    return false
               else
                    if player:removeItem(25172, 100) then
                         local itemAdd = player:addItem(items[skillChoice[cid]][element], 1)
                         npcHandler:say("Ah, excellent. Here is your " .. itemAdd:getName():lower() .. ".", cid)
                    else
                         npcHandler:say("Sorry, friend, but one good turn deserves another. Bring enough tokens and it's a deal.", cid)
                    end
                    skillChoice[cid] = nil
                    npcHandler.topic[cid] = 0
    return true

end
npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
end
end
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
