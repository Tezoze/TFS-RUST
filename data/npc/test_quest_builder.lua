-- Test Quest NPC using the new NpcBuilder framework
-- Uses storage key 99900 for testing (outside all existing ranges).
-- This validates QuestBuilder with storage-driven states and transitions.

local npc = QuestBuilder:new("Test Questgiver", {lookType = 130, lookHead = 78, lookBody = 69, lookLegs = 58, lookFeet = 76})
npc:greetMessage("Greetings, |PLAYERNAME|. I have nothing for you right now.")
npc:farewellMessage("Farewell, |PLAYERNAME|.")

-- State 1: Quest not started (storage 99900 == -1 or not set)
npc:addState({
    storage = {99900, -1},
    greetText = "Greetings, |PLAYERNAME|. I have a task for you. Will you help me?",
    transitions = {
        {
            keywords = {"yes"},
            text = "Excellent! Go kill 10 rats and return to me.",
            nextStorage = {99900, 1},
        },
        {
            keywords = {"no"},
            text = "Come back when you change your mind.",
        },
    }
})

-- State 2: Quest in progress (storage 99900 == 1)
npc:addState({
    storage = {99900, 1},
    greetText = "Have you killed the rats yet, |PLAYERNAME|?",
    transitions = {
        {
            keywords = {"yes"},
            text = "Well done! Here is your reward.",
            nextStorage = {99900, 2},
            actions = {
                { type = "giveItem", params = { itemId = 2160, count = 5 } }, -- 5 crystal coins
            },
        },
        {
            keywords = {"no"},
            text = "Keep trying, |PLAYERNAME|!",
        },
    }
})

-- State 3: Quest complete (storage 99900 == 2)
npc:addState({
    storage = {99900, 2},
    greetText = "Thank you again for your help, |PLAYERNAME|. You are a true hero.",
})

npc:addKeyword("job", "I am a test questgiver for the new NPC builder framework.")
npc:register()
