-- Santa Claus - Converted from XML to Lua NpcType
-- Original XML: data/npc/Santa Claus.xml
-- Original Script: data/npc/scripts/Santa Claus.lua

local npcName = "Santa Claus"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a santa claus")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 160, lookBody = 112, lookLegs = 93, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)
local talkState = {}


local normalItems = {
     {7439, 7440, 7443},
     {2688, 6508},
     {2688, 6509},
     {2688, 6507},
     {2688, 2114},
     {2688, 2111},
     {2167, 2213, 2214},
     {11227},
     {2156},
     {2153}
}

local semiRareItems = {
     {2173},
     {9954},
     {9971},
     {5080}
}

local rareItems = {
     {2110},
     {5919},
     {6567},
     {11255},
     {11256},
     {6566},
     {2112},
}

local veryRareItems = {
     {2659},
     {3954},
     {2644},
     {10521},
     {5804}
}

local function getReward()
     local rewardTable = {}
     local random = math.random(100)
     if (random <= 90) then
          rewardTable = normalItems
     elseif (random <= 70) then
          rewardTable = semiRareItems
     elseif (random <= 30) then
          rewardTable = rareItems
     elseif (random <= 10) then
          rewardTable = veryRareItems
     end

     local rewardItem = rewardTable[math.random(#rewardTable)]
     return rewardItem
end

function creatureSayCallback(cid, type, msg)
     if(not npcHandler:isFocused(cid)) then
          return false
     end
     local talkUser = NPCHANDLER_CONVBEHAVIOR == CONVERSATION_DEFAULT and 0 or cid

     if msgcontains(msg, 'present') then
          local player = Player(cid)
          if (player:getStorageValue(840293) > os.time()) then
               npcHandler:say("You can't get other present.", cid)
               return false
          end



          local reward = getReward()
          local cont = Container(Player(cid):addItem(6511):getUniqueId())
          local count = 1

          for i = 1, #reward do
               if (reward[i] == 2111 or
                   reward[i] == 2688) then
                    count = 10
               end

               cont:addItem(reward[i], count)
          end

          player:setStorageValue(840293, os.time() + 86400)
          npcHandler:say("Merry Christmas!", cid)
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
