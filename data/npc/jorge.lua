-- Jorge - Converted from XML to Lua NpcType
-- Original XML: data/npc/Jorge.xml
-- Original Script: data/npc/scripts/Jorge.lua

local npcName = "Jorge"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a jorge")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 159, lookHead = 38, lookBody = 77, lookLegs = 78, lookFeet = 94})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)
 

local items = {
     [1] = {name = "Abacus", id = 21467},
     [2] = {name = "Assassin Doll", id = 32594},
     [3] = {name = "Bag of Oriental Spices", id = 26338},
     [4] = {name = "Bookworm Doll", id = 32592},
     [5] = {name = "Cateroides Doll", id = 29217},
     [6] = {name = "Doll of Durin the Almighty", id = 26335},
     [7] = {name = "Dragon Eye", id = 24683},
     [8] = {name = "Dragon Goblet", id = 36100},
     [9] = {name = "Draken Doll", id = 29215},
     [10] = {name = "Encyclopedia", id = 26334},
     [11] = {name = "Friendship Amulet", id = 21469},
     [12] = {name = "Frozen Heart", id = 21472},
     [13] = {name = "Golden Falcon", id = 32593},
     [14] = {name = "Golden Newspaper", id = 26337},
     [15] = {name = "Hand Puppets", id = 26332},
     [16] = {name = "Imortus", id = 26339},
     [17] = {name = "Jade Amulet", id = 36103},
     [18] = {name = "Key of Numerous Locks", id = 21468},
     [19] = {name = "Loremaster Doll", id = 36102},
     [20] = {name = "Mathmaster Shield", id = 29218},
     [21] = {name = "Medusa Skull", id = 26336},
     [22] = {name = "Music Box", id = 26333},
     [23] = {name = "Noble Sword", id = 18551},
     [24] = {name = "Norsemal Doll", id = 21466},
     [25] = {name = "Old Radio", id = 32591},
     [26] = {name = "Orcs Jaw Shredder", id = 21471},
     [27] = {name = "Pigeon Trophy", id = 36101},
     [28] = {name = "Phoenix Statue", id = 24682},
     [29] = {name = "The Mexcalibur", id = 21470},
     [30] = {name = "TibiaHispano Emblem", id = 29216}
}

local function greetCallback(cid)
    return true
end

local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end
    
    local player = Player(cid)

    if msg then
        for i = 1, #items do
          if msgcontains(msg, items[i].name) then
                if getPlayerItemCount(cid, 21400) >= 20 then
                    doPlayerRemoveItem(cid, 21400, 20)
                    doPlayerAddItem(cid, items[i].id, 1)
                    selfSay('You just swapped 20 silver raid tokens for 1 '.. getItemName(items[i].name) ..'.', cid)
                else
                    selfSay('You need 20 silver raid tokens.', cid)
                end
            end
        end
    end
    return true
end

local function onAddFocus(cid)
end

local function onReleaseFocus(cid)
end

npcHandler:setCallback(CALLBACK_ONADDFOCUS, onAddFocus)
npcHandler:setCallback(CALLBACK_ONRELEASEFOCUS, onReleaseFocus)

npcHandler:setCallback(CALLBACK_GREET, greetCallback)
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
