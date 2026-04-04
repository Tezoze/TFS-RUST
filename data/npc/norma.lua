-- Norma - Converted from XML to Lua NpcType
-- Original XML: data/npc/Norma.xml
-- Original Script: data/npc/scripts/Norma.lua

local npcName = "Norma"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a norma")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 136, lookHead = 78, lookBody = 76, lookLegs = 72, lookFeet = 96, lookAddons = 2})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'Great drinks and snacks at fair prices!' },
	{ text = 'You know you want a party after all that tiring hunting!' },
	{ text = '<sings> ... are a girl\'s best friieeend...' },
	{ text = 'Sing and dance at my bar! Yeah!' },
	{ text = 'Best place in town! Come to my bar!' }
}
npcHandler:addModule(VoiceModule:new(voices))

--[[
addon
Pretty, isn't it? I made it myself, but I could teach you how to do that if you like. What do you say?
hat
13:44 Norma: Pretty, isn't it? I made it myself, but I could teach you how to do that if you like. What do you say?
yes
13:44 Norma: Okay, here we go, listen closely! I need a few things... a basic hat of course, maybe a legion helmet would do. Then about 100 chicken feathers... and 50 honeycombs as glue. That's it, come back to me once you gathered it!!
]]

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, "addon") or msgcontains(msg, "outfit") or msgcontains(msg, "hat") then
		local addonProgress = player:getStorageValue(Storage.OutfitQuest.Citizen.AddonHat)
		if addonProgress < 1 then
			npcHandler:say("Pretty, isn't it? My friend Amber taught me how to make it, but I could help you with one if you like. What do you say?", cid)
			npcHandler.topic[cid] = 1
		elseif addonProgress == 1 then
			npcHandler:say("Oh, you're back already? Did you bring a legion helmet, 100 chicken feathers and 50 honeycombs?", cid)
			npcHandler.topic[cid] = 2
		elseif addonProgress == 2 then
			npcHandler:say('Pretty hat, isn\'t it?', cid)
		end
		return true
	end

	if npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'yes') then
			player:setStorageValue(Storage.OutfitQuest.Ref, math.max(0, player:getStorageValue(Storage.OutfitQuest.Ref)) + 1)
			player:setStorageValue(Storage.OutfitQuest.Citizen.AddonHat, 1)
			player:setStorageValue(Storage.OutfitQuest.Citizen.MissionHat, 1)
			npcHandler:say('Okay, here we go, listen closely! I need a few things... a basic hat of course, maybe a legion helmet would do. Then about 100 chicken feathers... and 50 honeycombs as glue. That\'s it, come back to me once you gathered it!', cid)
		else
			npcHandler:say('Aw, I guess you don\'t like feather hats. No big deal.', cid)
		end
		npcHandler.topic[cid] = 0

	elseif npcHandler.topic[cid] == 2 then
		if msgcontains(msg, 'yes') then
			if player:getItemCount(2480) < 1 then
				npcHandler:say('Sorry, but I can\'t see a legion helmet.', cid)
			elseif player:getItemCount(5890) < 100 then
				npcHandler:say('Sorry, but you don\'t enough chicken feathers.', cid)
			elseif player:getItemCount(5902) < 50 then
				npcHandler:say('Sorry, but you don\'t have enough honeycombs.', cid)
			else
				npcHandler:say('Great job! That must have taken a lot of work. Okay, you put it like this... then glue like this... here!', cid)
				player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)

				player:removeItem(2480, 1)
				player:removeItem(5902, 50)
				player:removeItem(5890, 100)

				player:addOutfitAddon(136, 2)
				player:addOutfitAddon(128, 2)

				player:setStorageValue(Storage.OutfitQuest.Ref, math.min(0, player:getStorageValue(Storage.OutfitQuest.Ref) - 1))
				player:setStorageValue(Storage.OutfitQuest.Citizen.MissionHat, 0)
				player:setStorageValue(Storage.OutfitQuest.Citizen.AddonHat, 2)
			end
		else
			npcHandler:say('Maybe another time.', cid)
		end
		npcHandler.topic[cid] = 0
	end

	return true
end

-- Basic keywords
keywordHandler:addKeyword({'hint'}, StdModule.rookgaardHints, {npcHandler = npcHandler})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = 'Well, my name is {Norma}.'})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I used to be a merchant, but being a barkeeper is so much more fun. It\'s been my dream since I was a little girl.'})
keywordHandler:addKeyword({'time'}, StdModule.say, {npcHandler = npcHandler, text = 'It\'s about |TIME|. Time just flies, doesn\'t it?'})
keywordHandler:addKeyword({'merchant'}, StdModule.say, {npcHandler = npcHandler, text = 'To view the offers of a merchant, simply talk to him or her and ask for a {trade}. They will gladly show you their offers and also the things they buy from you.'})
keywordHandler:addKeyword({'how', 'are', 'you'}, StdModule.say, {npcHandler = npcHandler, text = 'I\'ve never been better! I love my new {job}!'})
keywordHandler:addKeyword({'weapon'}, StdModule.say, {npcHandler = npcHandler, text = 'No sorry, I\'m out of that business. Please ask {Obi} or {Lee\'Delle} if you need a weapon.'})
keywordHandler:addKeyword({'equipment'}, StdModule.say, {npcHandler = npcHandler, text = 'As an adventurer, you should always have at least a {backpack}, a {rope}, a {shovel}, a {weapon}, an {armor} and a {shield}.'})
keywordHandler:addKeyword({'monster'}, StdModule.say, {npcHandler = npcHandler, text = 'Ah, the monsters are far away. Let\'s party now!'})
keywordHandler:addKeyword({'dungeon'}, StdModule.say, {npcHandler = npcHandler, text = 'If you really want to talk about dungeons, visit Dallheim on the bridge near my bar.'})
keywordHandler:addKeyword({'sewer'}, StdModule.say, {npcHandler = npcHandler, text = 'Ewww, I don\'t even want to think about the smell from the sewers right now!'})
keywordHandler:addKeyword({'king'}, StdModule.say, {npcHandler = npcHandler, text = 'Oooh, I wouldn\'t dare gossiping about the king. He might have his spies here somewhere, you know.'})
keywordHandler:addKeyword({'tibia'}, StdModule.say, {npcHandler = npcHandler, text = 'This is the world we live in, oh-oh-oh, and these are the hands we\'re given, oh-oh... oh, I\'m sorry, I got carried away!'})
keywordHandler:addKeyword({'premium'}, StdModule.say, {npcHandler = npcHandler, text = 'As a premium adventurer you have many advantages, you should check them out!'})
keywordHandler:addKeyword({'drink'}, StdModule.say, {npcHandler = npcHandler, text = 'I serve the best drinks in town! What\'s it gonna be, lemonade? Wine? Milk? Beer? Just ask me for a {trade}!'})
keywordHandler:addKeyword({'temple'}, StdModule.say, {npcHandler = npcHandler, text = 'The temple is a good place to visit if you\'re in urgent need for healing.'})

keywordHandler:addKeyword({'help'}, StdModule.say, {npcHandler = npcHandler, text = 'Well, the only help I could give you are some general {hints}. And a delicious {drink} along with it.'})
keywordHandler:addAliasKeyword({'information'})

keywordHandler:addKeyword({'backpack'}, StdModule.say, {npcHandler = npcHandler, text = 'No sorry, I\'m out of that business. Please ask {Al Dee} or {Lee\'Delle} if you need equipment.'})
keywordHandler:addAliasKeyword({'rope'})
keywordHandler:addAliasKeyword({'shovel'})
keywordHandler:addAliasKeyword({'fishing'})

keywordHandler:addKeyword({'armor'}, StdModule.say, {npcHandler = npcHandler, text = 'No sorry, I\'m out of that business. Please ask {Dixi} or {Lee\'Delle} if you need equipment.'})
keywordHandler:addAliasKeyword({'shield'})

keywordHandler:addKeyword({'offer'}, StdModule.say, {npcHandler = npcHandler, text = 'Oh, would you like something to eat or drink? Ask me for a {trade} to see my offers!'})
keywordHandler:addAliasKeyword({'sell'})
keywordHandler:addAliasKeyword({'buy'})
keywordHandler:addAliasKeyword({'wares'})
keywordHandler:addAliasKeyword({'stuff'})

-- Names
keywordHandler:addKeyword({'mary'}, StdModule.say, {npcHandler = npcHandler, text = '<gets a dreamy look in her eyes>'})
keywordHandler:addKeyword({'al', 'dee'}, StdModule.say, {npcHandler = npcHandler, text = 'Al Dee treats his customers friendly, but he\'s badmouthing them once he had a few beers.'})
keywordHandler:addKeyword({'amber'}, StdModule.say, {npcHandler = npcHandler, text = 'I\'m curious about her adventures. She really should come here more often!'})
keywordHandler:addKeyword({'billy'}, StdModule.say, {npcHandler = npcHandler, text = 'He got lucky when Willie and him played a game of dice about who gets to farm on which side of the river. Ever since, Willie holds a grudge against him.'})
keywordHandler:addKeyword({'willie'}, StdModule.say, {npcHandler = npcHandler, text = 'Hehe, that guy doesn\'t pretend to be anyone that he isn\'t. He is what he is - rude, but honest.'})
keywordHandler:addKeyword({'tom'}, StdModule.say, {npcHandler = npcHandler, text = 'Oh shush! It\'s kinda disgusting when he walks in here with animal blood on his hands!'})
keywordHandler:addKeyword({'seymour'}, StdModule.say, {npcHandler = npcHandler, text = 'What Seymour doesn\'t teach you is how much fun you can have here!'})
keywordHandler:addKeyword({'zirella'}, StdModule.say, {npcHandler = npcHandler, text = 'I invited her to my bar, but she refused. Oh well, it was worth a try!'})
keywordHandler:addKeyword({'santiago'}, StdModule.say, {npcHandler = npcHandler, text = 'I think that poor guy never gets a break from work. At least he never comes here.'})
keywordHandler:addKeyword({'paulie'}, StdModule.say, {npcHandler = npcHandler, text = 'That guy seriously needs some partying and a girlfriend.'})
keywordHandler:addKeyword({'oracle'}, StdModule.say, {npcHandler = npcHandler, text = 'The oracle is on the top floor of the academy. You should go there once you are level 8 to leave this island.'})
keywordHandler:addKeyword({'obi'}, StdModule.say, {npcHandler = npcHandler, text = 'I think it\'s rather strange that he lives here with his granddaughter. No one\'s ever seen the parents of {Dixi}.'})
keywordHandler:addKeyword({'norma'}, StdModule.say, {npcHandler = npcHandler, text = 'I hate that name, it sounds so boring. Almost like \'normal\'. I think I should change my name to something more exciting. Like... Marylin! Oh, what a glamorous sound!'})
keywordHandler:addKeyword({'dixi'}, StdModule.say, {npcHandler = npcHandler, text = 'She told me that she dreams of leaving this island to go on a big adventure.'})
keywordHandler:addKeyword({'loui'}, StdModule.say, {npcHandler = npcHandler, text = 'They tell me strange stories about him and some monsters in a hole. Weird!'})
keywordHandler:addKeyword({'lee\'delle'}, StdModule.say, {npcHandler = npcHandler, text = 'That girl thinks that she is soooo special and \'high society\'. We used to be friends, but well, things change.'})
keywordHandler:addKeyword({'hyacinth'}, StdModule.say, {npcHandler = npcHandler, text = 'That\'s probably someone I\'ll never see in my bar.'})
keywordHandler:addKeyword({'cipfried'}, StdModule.say, {npcHandler = npcHandler, text = '<giggles> That monk with his holier-than-thou attitude hasn\'t anything against a good mug of wine now and then.'})
keywordHandler:addKeyword({'dallheim'}, StdModule.say, {npcHandler = npcHandler, text = 'If you listen to him, you could get the impression my little bar is the devil himself. He says he has more work now keeping drunken adventurers in than monsters out of the city. Such an exaggeration!'})
keywordHandler:addAliasKeyword({'zerbrus'})

npcHandler:setMessage(MESSAGE_WALKAWAY, 'Hey, where are you going? We\'ve just started!')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Come back soon!')
npcHandler:setMessage(MESSAGE_SENDTRADE, 'Take all the time you need to decide what you want!')
npcHandler:setMessage(MESSAGE_GREET, 'Welcome, welcome! Have a seat! If you like a drink or something to eat, just ask me for a {trade}!')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2006, buy = 10, sell = 0, subType = 15, name = "wine"},
    {id = 2689, buy = 3, sell = 0, subType = 0, name = "bread"},
    {id = 2696, buy = 5, sell = 0, subType = 0, name = "cheese"},
    {id = 2695, buy = 2, sell = 0, subType = 0, name = "egg"},
    {id = 2012, buy = 2, sell = 0, subType = 5, name = "mug of lemonade"},
    {id = 6280, buy = 50, sell = 0, subType = 0, name = "party cake"},
}

-- Helper function to find shop item by id and subType (for fluid containers)
local function getShopItem(itemId, subType, isBuying)
    local itemType = ItemType(itemId)
    if itemType:isFluidContainer() then
        for _, item in ipairs(shopItems) do
            if item.id == itemId and item.subType == subType then
                return item
            end
        end
    end
    -- For non-fluid items, find the entry that matches the operation
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            if isBuying and item.buy > 0 then
                return item
            elseif not isBuying and item.sell > 0 then
                return item
            end
        end
    end
    -- Fallback to first match
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            return item
        end
    end
    return nil
end

local function openTradeWindow(cid, message, keywords, parameters, node)
    if not npcHandler:isFocused(cid) then return false end
    local player = Player(cid)
    if not player then return false end
    local npc = Npc(getNpcCid())
    local shopList = {}
    for _, item in ipairs(shopItems) do
        table.insert(shopList, {id = item.id, buy = item.buy, sell = item.sell, subType = item.subType or 0, name = item.name})
    end
    npc:openShopWindow(player, shopList, function() return true end, function() return true end)
    npcHandler:say('Take all the time you need to browse my wares.', cid)
    return true
end
keywordHandler:addKeyword({'trade'}, openTradeWindow, {npcHandler = npcHandler})


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

npcType:eventType(NPCS_EVENT_BUYITEM)
npcType:onBuyItem(function(npc, player, itemId, subType, amount, ignoreCap, inBackpacks)
    local shopItem = getShopItem(itemId, subType, true)
    if not shopItem or shopItem.buy <= 0 then return false end
    local totalCost = amount * shopItem.buy
    if player:getTotalMoney() < totalCost then
        player:sendCancelMessage("You don't have enough money.")
        return false
    end
    local itemSubType = shopItem.subType or 1
    local bought = doNpcSellItem(player:getId(), itemId, amount, itemSubType, ignoreCap, inBackpacks, ITEM_BACKPACK)
    if bought == 0 then
        player:sendCancelMessage("You do not have enough capacity.")
        return false
    end
    player:removeTotalMoney(bought * shopItem.buy)
    player:sendTextMessage(MESSAGE_INFO_DESCR, "Bought " .. bought .. "x " .. shopItem.name .. " for " .. (bought * shopItem.buy) .. " gold.")
    return true
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
