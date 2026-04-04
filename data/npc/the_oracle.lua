-- The Oracle - Converted from XML to Lua NpcType
-- Original XML: data/npc/The Oracle.xml
-- Original Script: data/npc/scripts/The Oracle.lua

local npcName = "The Oracle"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a the oracle")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookTypeEx = 1448})
npcType:speechBubble(SPEECHBUBBLE_3)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)

local vocation = {}
local town = {}
local config = {
    towns = {
        ["venore"] = 1,
        ["thais"] = 2,
        ["kazordoon"] = 3,
        ["carlin"] = 4,
        ["ab'dendriel"] = 5,
        ["liberty bay"] = 7,
        ["port hope"] = 8,
        ["ankrahmun"] = 9,
        ["darashia"] = 10,
        ["edron"] = 11,
        ["svargrond"] = 12
    },

    vocations = {
        ["sorcerer"] = {
            text = "A SORCERER! ARE YOU SURE? THIS DECISION IS IRREVERSIBLE!",
            vocationId = 1
        },

        ["druid"] = {
            text = "A DRUID! ARE YOU SURE? THIS DECISION IS IRREVERSIBLE!",
            vocationId = 2
        },

        ["paladin"] = {
            text = "A PALADIN! ARE YOU SURE? THIS DECISION IS IRREVERSIBLE!",
            vocationId = 3
        },

        ["knight"] = {
            text = "A KNIGHT! ARE YOU SURE? THIS DECISION IS IRREVERSIBLE!",
            vocationId = 4
        }
    }
}


local function greetCallback(cid)
    local player = Player(cid)
    local level = player:getLevel()
    
    if level < 8 then
        npcHandler:say("CHILD! COME BACK WHEN YOU HAVE GROWN UP!", cid)
        npcHandler:resetNpc(cid)
        return false
    elseif player:getVocation():getId() > 0 then
        npcHandler:say("YOU ALREADY HAVE A VOCATION!", cid)
        npcHandler:resetNpc(cid)
        return false
    else
        npcHandler:setMessage(MESSAGE_GREET, player:getName() ..", ARE YOU PREPARED TO FACE YOUR DESTINY?")
    end
    return true
end

local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end

    local player = Player(cid)
    
    -- Variables for the long town string
    local townList = "{AB'DENDRIEL}, {ANKRAHMUN}, {CARLIN}, {DARASHIA}, {EDRON}, {KAZORDOON}, {LIBERTY BAY}, {PORT HOPE}, {SVARGROND}, {THAIS}, OR {VENORE}"

    if npcHandler.topic[cid] == 0 then
        if msgcontains(msg, "yes") then
            npcHandler:say("IN WHICH TOWN DO YOU WANT TO LIVE: " .. townList .. "?", cid)
            npcHandler.topic[cid] = 1
        end
    elseif npcHandler.topic[cid] == 1 then
        local cityTable = config.towns[msg:lower()]
        if cityTable then
            town[cid] = cityTable
            npcHandler:say("IN ".. string.upper(msg) .."! AND WHAT PROFESSION HAVE YOU CHOSEN: {KNIGHT}, {PALADIN}, {SORCERER}, OR {DRUID}?", cid)
            npcHandler.topic[cid] = 2
        else
            npcHandler:say("IN WHICH TOWN DO YOU WANT TO LIVE: " .. townList .. "?", cid)
        end
    elseif npcHandler.topic[cid] == 2 then
        local vocationTable = config.vocations[msg:lower()]
        if vocationTable then
            npcHandler:say(vocationTable.text, cid)
            npcHandler.topic[cid] = 3
            vocation[cid] = vocationTable.vocationId
        else
            npcHandler:say("{KNIGHT}, {PALADIN}, {SORCERER}, OR {DRUID}?", cid)
        end
    elseif npcHandler.topic[cid] == 3 then
        if msgcontains(msg, "yes") then
            npcHandler:say("SO BE IT!", cid)
            player:setVocation(Vocation(vocation[cid]))
            player:setTown(Town(town[cid]))
            player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
            player:teleportTo(Town(town[cid]):getTemplePosition())
            player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
        else
            npcHandler:say("THEN WHAT? {KNIGHT}, {PALADIN}, {SORCERER}, OR {DRUID}?", cid)
            npcHandler.topic[cid] = 2
        end
    end
    return true
end

local function onAddFocus(cid)
    town[cid] = 0
    vocation[cid] = 0
end

local function onReleaseFocus(cid)
    town[cid] = nil
    vocation[cid] = nil
end

npcHandler:setCallback(CALLBACK_ONADDFOCUS, onAddFocus)
npcHandler:setCallback(CALLBACK_ONRELEASEFOCUS, onReleaseFocus)
npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setMessage(MESSAGE_FAREWELL, "COME BACK WHEN YOU ARE PREPARED TO FACE YOUR DESTINY!")
npcHandler:setMessage(MESSAGE_WALKAWAY, "COME BACK WHEN YOU ARE PREPARED TO FACE YOUR DESTINY!")
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
