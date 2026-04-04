-- Wyrdin - Converted from XML to Lua NpcType
-- Original XML: data/npc/Wyrdin.xml
-- Original Script: data/npc/scripts/Wyrdin.lua

local npcName = "Wyrdin"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a wyrdin")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookHead = 76, lookBody = 77, lookLegs = 79, lookFeet = 115})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
    { text = "<mumbles> So where was I again?" },
    { text = "<mumbles> Typical - you can never find a hero when you need one!" },
    { text = "<mumbles> Could the bonelord language be the invention of some madman?" },
    { text = "<mumbles> The curse algorithm of triplex shadowing has to be two times higher than an overcharged nanoquorx on the peripheral..." }
}
npcHandler:addModule(VoiceModule:new(voices))

local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end

    local player = Player(cid)
    local yalaharStorage = player:getStorageValue(Storage.TheWayToYalahar.QuestLine)
    local explorerStorage = player:getStorageValue(Storage.ExplorerSociety.QuestLine) -- 4 = Novice Rank

    -- THE WAY TO YALAHAR QUEST
    if msgcontains(msg, "mission") then
        -- Attempting to start the quest
        if yalaharStorage < 1 then
            -- CHECK: Must be at least Novice in Explorer Society
            if explorerStorage >= 4 then
                npcHandler:say({
                    "There is indeed something that needs our attention. In the far north, a new city named Yalahar was discovered. It seems to be incredibly huge. ...",
                    "According to travelers, it's a city of glory and wonders. We need to learn as much as we can about this city and its inhabitants. ...",
                    "Gladly the explorer's society already sent a representative there. Still, we need someone to bring us the information he was able to gather until now. ...",
                    "Please look for the explorer's society's captain Maximilian in Liberty Bay. Ask him for a passage to Yalahar. There visit Timothy of the explorer's society and get his research notes. ...",
                    "It might be a good idea to explore the city a bit on your own before you deliver the notes here, but please make sure you don't lose them."
                }, cid)
                player:setStorageValue(Storage.TheWayToYalahar.QuestLine, 1)
            else
                -- Rejection message if not in Explorer Society
                npcHandler:say("I indeed have a mission that requires travelling, but I cannot entrust it to an amateur. Please join the Explorer Society in Port Hope first and prove your worth as a traveller.", cid)
            end
            npcHandler.topic[cid] = 0
            return true

        -- Turning in the research notes
        elseif yalaharStorage == 2 then
            npcHandler:say("Did you bring the papers I asked you for?", cid)
            npcHandler.topic[cid] = 1
            return true
        
        -- Already completed
        elseif yalaharStorage >= 3 then
            npcHandler:say("You have already helped me enough with Yalahar. Thank you.", cid)
            npcHandler.topic[cid] = 0
            return true
        end

    -- Yalahar Quest Confirmation (Turning in notes)
    elseif msgcontains(msg, "yes") and npcHandler.topic[cid] == 1 then
        if player:removeItem(10090, 1) then -- Research Notes
            player:setStorageValue(Storage.TheWayToYalahar.QuestLine, 3)
            player:addMoney(500)
            npcHandler:say("Oh marvellous, please excuse me. I need to read this text immediately. Here, take this small reward of 500 gold pieces for your efforts.", cid)
        else
            npcHandler:say("You don't have the research notes from Timothy.", cid)
        end
        npcHandler.topic[cid] = 0
        return true

    -- THE NEW FRONTIER QUEST
    elseif msgcontains(msg, "farmine") then
        if player:getStorageValue(Storage.TheNewFrontier.Questline) == 15 then
            npcHandler:say("I've heard some odd rumours about this new dwarven outpost. But tell me, what has the Edron academy to do with Farmine?", cid)
            npcHandler.topic[cid] = 2
            return true
        end

    elseif msgcontains(msg, "plea") then
        if npcHandler.topic[cid] == 2 then
            if player:getStorageValue(Storage.TheNewFrontier.BribeWydrin) < 1 then
                npcHandler:say("Hm, you are right, we are at the forefront of knowledge and innovation. Our dwarven friends could learn much from one of our representatives.", cid)
                player:setStorageValue(Storage.TheNewFrontier.BribeWydrin, 1)
                -- Update Questlog: "Mission 05: Getting Things Busy"
                player:setStorageValue(Storage.TheNewFrontier.Mission05, player:getStorageValue(Storage.TheNewFrontier.Mission05) + 1) 
            end
            npcHandler.topic[cid] = 0
            return true
        end
    end

    return true
end

npcHandler:setMessage(MESSAGE_GREET, "Hello, what brings you here?")

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
