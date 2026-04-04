-- Yasir - Converted from XML to Lua NpcType
-- Original XML: data/npc/Yasir.xml
-- Original Script: data/npc/scripts/Yasir.lua

local npcName = "Yasir"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a yasir")
npcType:health(150)
npcType:maxHealth(150)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 146, lookHead = 85, lookBody = 7, lookLegs = 12, lookFeet = 19, lookAddons = 2})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if(not npcHandler:isFocused(cid)) then
	return false
	end

	if msg:lower() == "name" then
		return npcHandler:say("Me Yasir.", cid)
	elseif msg:lower() == "job" then
		return npcHandler:say("Tje hari ku ne finjala. {Ariki}?", cid)
	elseif msg:lower() == "passage" then
		return npcHandler:say("Soso yana. <shakes his head>", cid)
	end
	return true
end

npcHandler:setMessage(MESSAGE_FAREWELL, "Si, jema ze harun. <waves>")
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 11213, buy = 0, sell = 10, subType = 0, name = "acorn"},
    {id = 34581, buy = 0, sell = 270000, subType = 0, name = "alptramun's toothbrush"},
    {id = 27052, buy = 0, sell = 260, subType = 0, name = "ancient belt buckle"},
    {id = 36423, buy = 0, sell = 28000, subType = 0, name = "ancient liche bone"},
    {id = 11214, buy = 0, sell = 50, subType = 0, name = "antlers"},
    {id = 5883, buy = 0, sell = 120, subType = 0, name = "ape fur"},
    {id = 7965, buy = 0, sell = 15, subType = 0, name = "badger fur"},
    {id = 12401, buy = 0, sell = 30, subType = 0, name = "bamboo stick"},
    {id = 12467, buy = 0, sell = 55, subType = 0, name = "banana sash"},
    {id = 6492, buy = 0, sell = 2000, subType = 0, name = "bat decoration"},
    {id = 5894, buy = 0, sell = 50, subType = 0, name = "bat wing"},
    {id = 5896, buy = 0, sell = 100, subType = 0, name = "bear paw"},
    {id = 34584, buy = 0, sell = 630000, subType = 0, name = "beast's nightmare-cushion"},
    {id = 29039, buy = 0, sell = 500, subType = 0, name = "bed of nails"},
    {id = 27049, buy = 0, sell = 200, subType = 0, name = "beetle carapace"},
    {id = 5930, buy = 0, sell = 2000, subType = 0, name = "behemoth claw"},
    {id = 10562, buy = 0, sell = 190, subType = 0, name = "black hood"},
    {id = 12404, buy = 0, sell = 300, subType = 0, name = "black wool"},
    {id = 12405, buy = 0, sell = 320, subType = 0, name = "blood preservation"},
    {id = 10550, buy = 0, sell = 100, subType = 0, name = "bloody pincers"},
    {id = 33983, buy = 0, sell = 60, subType = 0, name = "blue glass plate"},
    {id = 36394, buy = 0, sell = 230, subType = 0, name = "blue goanna scale"},
    {id = 5912, buy = 0, sell = 200, subType = 0, name = "blue piece of cloth"},
    {id = 10584, buy = 0, sell = 200, subType = 0, name = "boggy dreads"},
    {id = 11321, buy = 0, sell = 150, subType = 0, name = "bone shoulderplate"},
    {id = 27048, buy = 0, sell = 150, subType = 0, name = "bone toothpick"},
    {id = 5898, buy = 0, sell = 80, subType = 0, name = "bonelord eye"},
    {id = 27610, buy = 0, sell = 10000, subType = 0, name = "bones of zorvorax"},
    {id = 11194, buy = 0, sell = 210, subType = 0, name = "bony tail"},
    {id = 11237, buy = 0, sell = 180, subType = 0, name = "book of necromantic rituals"},
    {id = 10563, buy = 0, sell = 120, subType = 0, name = "book of prayers"},
    {id = 33316, buy = 0, sell = 640, subType = 0, name = "book page"},
    {id = 22538, buy = 0, sell = 500, subType = 0, name = "bowl of terror sweat"},
    {id = 35151, buy = 0, sell = 220, subType = 0, name = "bright bell"},
    {id = 12658, buy = 0, sell = 380, subType = 0, name = "brimstone fangs"},
    {id = 12659, buy = 0, sell = 210, subType = 0, name = "brimstone shell"},
    {id = 35044, buy = 0, sell = 150, subType = 0, name = "broken bell"},
    {id = 12407, buy = 0, sell = 30, subType = 0, name = "broken crossbow"},
    {id = 12616, buy = 0, sell = 340, subType = 0, name = "broken draken mail"},
    {id = 11335, buy = 0, sell = 100, subType = 0, name = "broken halberd"},
    {id = 12409, buy = 0, sell = 20, subType = 0, name = "broken helmet"},
    {id = 12608, buy = 0, sell = 8000, subType = 0, name = "broken key ring"},
    {id = 12408, buy = 0, sell = 35, subType = 0, name = "broken shamanic staff"},
    {id = 12617, buy = 0, sell = 120, subType = 0, name = "broken slicer"},
    {id = 22518, buy = 0, sell = 1900, subType = 0, name = "broken visor"},
    {id = 5913, buy = 0, sell = 100, subType = 0, name = "brown piece of cloth"},
    {id = 10606, buy = 0, sell = 30, subType = 0, name = "bunch of troll hair"},
    {id = 10605, buy = 0, sell = 800, subType = 0, name = "bundle of cursed straw"},
    {id = 11217, buy = 0, sell = 50, subType = 0, name = "carniphila seeds"},
    {id = 11192, buy = 0, sell = 35, subType = 0, name = "carrion worm fang"},
    {id = 5480, buy = 0, sell = 2000, subType = 0, name = "cat's paw"},
    {id = 30834, buy = 0, sell = 550, subType = 0, name = "cave devourer eyes"},
    {id = 30836, buy = 0, sell = 350, subType = 0, name = "cave devourer legs"},
    {id = 30835, buy = 0, sell = 600, subType = 0, name = "cave devourer maw"},
    {id = 11218, buy = 0, sell = 28, subType = 0, name = "centipede leg"},
    {id = 30838, buy = 0, sell = 240, subType = 0, name = "chasm spawn abdomen"},
    {id = 30837, buy = 0, sell = 850, subType = 0, name = "chasm spawn head"},
    {id = 30839, buy = 0, sell = 120, subType = 0, name = "chasm spawn tail"},
    {id = 20098, buy = 0, sell = 150, subType = 0, name = "cheesy figurine"},
    {id = 5890, buy = 0, sell = 30, subType = 0, name = "chicken feather"},
    {id = 30857, buy = 0, sell = 10000, subType = 0, name = "chitinous mouth"},
    {id = 30861, buy = 0, sell = 10000, subType = 0, name = "chitinous mouth"},
    {id = 36513, buy = 0, sell = 650, subType = 0, name = "cobra crest"},
    {id = 10551, buy = 0, sell = 15, subType = 0, name = "cobra tongue"},
    {id = 12470, buy = 0, sell = 110, subType = 0, name = "colourful feather"},
    {id = 27757, buy = 0, sell = 400, subType = 0, name = "colourful feathers"},
    {id = 29001, buy = 0, sell = 250, subType = 0, name = "colourful snail shell"},
    {id = 11219, buy = 0, sell = 45, subType = 0, name = "compass"},
    {id = 26157, buy = 0, sell = 260, subType = 0, name = "condensed energy"},
    {id = 11326, buy = 0, sell = 700, subType = 0, name = "corrupted flag"},
    {id = 6536, buy = 0, sell = 50000, subType = 0, name = "countess sorrow's frozen tear"},
    {id = 11189, buy = 0, sell = 35, subType = 0, name = "crab pincers"},
    {id = 27053, buy = 0, sell = 180, subType = 0, name = "cracked alabaster vase"},
    {id = 26177, buy = 0, sell = 250, subType = 0, name = "crystal bone"},
    {id = 26163, buy = 0, sell = 400, subType = 0, name = "crystallized anger"},
    {id = 10555, buy = 0, sell = 280, subType = 0, name = "cultish mask"},
    {id = 10556, buy = 0, sell = 150, subType = 0, name = "cultish robe"},
    {id = 12411, buy = 0, sell = 500, subType = 0, name = "cultish symbol"},
    {id = 26167, buy = 0, sell = 430, subType = 0, name = "curious matter"},
    {id = 11327, buy = 0, sell = 320, subType = 0, name = "cursed shoulder spikes"},
    {id = 10574, buy = 0, sell = 55, subType = 0, name = "cyclops toe"},
    {id = 32519, buy = 0, sell = 280, subType = 0, name = "damaged armor plates"},
    {id = 30855, buy = 0, sell = 8000, subType = 0, name = "damaged worm head"},
    {id = 29000, buy = 0, sell = 200, subType = 0, name = "dandelion seeds"},
    {id = 26171, buy = 0, sell = 300, subType = 0, name = "dangerous proto matter"},
    {id = 35152, buy = 0, sell = 250, subType = 0, name = "dark bell"},
    {id = 11220, buy = 0, sell = 48, subType = 0, name = "dark rosary"},
    {id = 22536, buy = 0, sell = 450, subType = 0, name = "dead weight"},
    {id = 15423, buy = 0, sell = 230, subType = 0, name = "deepling guard belt buckle"},
    {id = 30829, buy = 0, sell = 500, subType = 0, name = "deepworm jaws"},
    {id = 30828, buy = 0, sell = 650, subType = 0, name = "deepworm spike roots"},
    {id = 30827, buy = 0, sell = 800, subType = 0, name = "deepworm spikes"},
    {id = 5527, buy = 0, sell = 300, subType = 0, name = "demon dust"},
    {id = 5954, buy = 0, sell = 1000, subType = 0, name = "demon horn"},
    {id = 10564, buy = 0, sell = 80, subType = 0, name = "demonic skeletal hand"},
    {id = 30832, buy = 0, sell = 350, subType = 0, name = "diremaw brainpan"},
    {id = 30833, buy = 0, sell = 270, subType = 0, name = "diremaw legs"},
    {id = 12412, buy = 0, sell = 120, subType = 0, name = "dirty turban"},
    {id = 12640, buy = 0, sell = 20, subType = 0, name = "downy feather"},
    {id = 6546, buy = 0, sell = 50000, subType = 0, name = "dracola's eye"},
    {id = 9948, buy = 0, sell = 5000, subType = 0, name = "dracoyle statue"},
    {id = 27605, buy = 0, sell = 700, subType = 0, name = "dragon blood"},
    {id = 5919, buy = 0, sell = 8000, subType = 0, name = "dragon claw"},
    {id = 11361, buy = 0, sell = 175, subType = 0, name = "dragon priest's wandtip"},
    {id = 27606, buy = 0, sell = 550, subType = 0, name = "dragon tongue"},
    {id = 12413, buy = 0, sell = 100, subType = 0, name = "dragon's tail"},
    {id = 12614, buy = 0, sell = 550, subType = 0, name = "draken sulphur"},
    {id = 12615, buy = 0, sell = 430, subType = 0, name = "draken wristbands"},
    {id = 34643, buy = 0, sell = 205, subType = 0, name = "dream essence egg"},
    {id = 15622, buy = 0, sell = 130, subType = 0, name = "dung ball"},
    {id = 11193, buy = 0, sell = 150, subType = 0, name = "elder bonelord tentacle"},
    {id = 12421, buy = 0, sell = 90, subType = 0, name = "elven astral observer"},
    {id = 12420, buy = 0, sell = 50, subType = 0, name = "elven scouting glass"},
    {id = 10552, buy = 0, sell = 45, subType = 0, name = "elvish talisman"},
    {id = 36166, buy = 0, sell = 270, subType = 0, name = "empty honey glass"},
    {id = 5891, buy = 0, sell = 20000, subType = 0, name = "enchanted chicken wing"},
    {id = 26179, buy = 0, sell = 300, subType = 0, name = "energy ball"},
    {id = 11223, buy = 0, sell = 360, subType = 0, name = "essence of a bad dream"},
    {id = 12627, buy = 0, sell = 390, subType = 0, name = "eye of corruption"},
    {id = 36278, buy = 0, sell = 950, subType = 0, name = "fafnar symbol"},
    {id = 28999, buy = 0, sell = 200, subType = 0, name = "fairy wings"},
    {id = 32520, buy = 0, sell = 650, subType = 0, name = "falcon crest"},
    {id = 2801, buy = 0, sell = 20, subType = 0, name = "fern"},
    {id = 10553, buy = 0, sell = 375, subType = 0, name = "fiery heart"},
    {id = 29043, buy = 0, sell = 200, subType = 0, name = "fig leaf"},
    {id = 5895, buy = 0, sell = 150, subType = 0, name = "fish fin"},
    {id = 12422, buy = 0, sell = 30, subType = 0, name = "flask of embalming fluid"},
    {id = 5885, buy = 0, sell = 10000, subType = 0, name = "flask of warrior's sweat"},
    {id = 30697, buy = 0, sell = 100, subType = 0, name = "fox paw"},
    {id = 22532, buy = 0, sell = 700, subType = 0, name = "frazzle tongue"},
    {id = 10575, buy = 0, sell = 160, subType = 0, name = "frost giant pelt"},
    {id = 10565, buy = 0, sell = 30, subType = 0, name = "frosty ear of a troll"},
    {id = 10578, buy = 0, sell = 280, subType = 0, name = "frosty heart"},
    {id = 26175, buy = 0, sell = 270, subType = 0, name = "frozen lightning"},
    {id = 10566, buy = 0, sell = 90, subType = 0, name = "gauze bandage"},
    {id = 12414, buy = 0, sell = 80, subType = 0, name = "geomancer's robe"},
    {id = 12419, buy = 0, sell = 120, subType = 0, name = "geomancer's staff"},
    {id = 11366, buy = 0, sell = 700, subType = 0, name = "ghastly dragon head"},
    {id = 10607, buy = 0, sell = 90, subType = 0, name = "ghostly tissue"},
    {id = 12423, buy = 0, sell = 60, subType = 0, name = "ghoul snack"},
    {id = 11197, buy = 0, sell = 380, subType = 0, name = "giant eye"},
    {id = 23570, buy = 0, sell = 170, subType = 0, name = "giant pacifier"},
    {id = 30854, buy = 0, sell = 10000, subType = 0, name = "giant tentacle"},
    {id = 12399, buy = 0, sell = 30, subType = 0, name = "girlish hair decoration"},
    {id = 8971, buy = 0, sell = 500, subType = 0, name = "gland"},
    {id = 26178, buy = 0, sell = 250, subType = 0, name = "glistening bone"},
    {id = 9967, buy = 0, sell = 25, subType = 0, name = "glob of acid slime"},
    {id = 9966, buy = 0, sell = 20, subType = 0, name = "glob of mercury"},
    {id = 9968, buy = 0, sell = 30, subType = 0, name = "glob of tar"},
    {id = 33317, buy = 0, sell = 350, subType = 0, name = "glowing rune"},
    {id = 36396, buy = 0, sell = 260, subType = 0, name = "goanna claw"},
    {id = 36395, buy = 0, sell = 190, subType = 0, name = "goanna meat"},
    {id = 12495, buy = 0, sell = 20, subType = 0, name = "goblin ear"},
    {id = 28995, buy = 0, sell = 250, subType = 0, name = "golden brush"},
    {id = 24630, buy = 0, sell = 270, subType = 0, name = "golden lotus brooch"},
    {id = 36159, buy = 0, sell = 38000, subType = 0, name = "golden mask"},
    {id = 32521, buy = 0, sell = 950, subType = 0, name = "grant of arms"},
    {id = 29002, buy = 0, sell = 500, subType = 0, name = "little bowl of myrrh"},
    {id = 5877, buy = 0, sell = 100, subType = 0, name = "green dragon leather"},
    {id = 5920, buy = 0, sell = 100, subType = 0, name = "green dragon scale"},
    {id = 33984, buy = 0, sell = 180, subType = 0, name = "green glass plate"},
    {id = 5910, buy = 0, sell = 200, subType = 0, name = "green piece of cloth"},
    {id = 29046, buy = 0, sell = 200, subType = 0, name = "guidebook"},
    {id = 12402, buy = 0, sell = 350, subType = 0, name = "hair of a banshee"},
    {id = 11200, buy = 0, sell = 55, subType = 0, name = "half-digested piece of meat"},
    {id = 30604, buy = 0, sell = 40, subType = 0, name = "half-digested stones"},
    {id = 10576, buy = 0, sell = 85, subType = 0, name = "half-eaten brain"},
    {id = 5925, buy = 0, sell = 70, subType = 0, name = "hardened bone"},
    {id = 30860, buy = 0, sell = 15000, subType = 0, name = "harpoon of a giant snail"},
    {id = 10600, buy = 0, sell = 115, subType = 0, name = "haunted piece of wood"},
    {id = 2743, buy = 0, sell = 50, subType = 0, name = "heaven blossom"},
    {id = 10554, buy = 0, sell = 500, subType = 0, name = "hellhound slobber"},
    {id = 11221, buy = 0, sell = 475, subType = 0, name = "hellspawn tail"},
    {id = 22540, buy = 0, sell = 350, subType = 0, name = "hemp rope"},
    {id = 11332, buy = 0, sell = 550, subType = 0, name = "high guard flag"},
    {id = 11333, buy = 0, sell = 130, subType = 0, name = "high guard shoulderplates"},
    {id = 5922, buy = 0, sell = 90, subType = 0, name = "holy orchid"},
    {id = 5902, buy = 0, sell = 40, subType = 0, name = "honeycomb"},
    {id = 27609, buy = 0, sell = 10000, subType = 0, name = "horn of kalyassa"},
    {id = 30856, buy = 0, sell = 15000, subType = 0, name = "huge shell"},
    {id = 30862, buy = 0, sell = 8000, subType = 0, name = "huge spiky snail shell"},
    {id = 12425, buy = 0, sell = 80, subType = 0, name = "hunter's quiver"},
    {id = 11199, buy = 0, sell = 600, subType = 0, name = "hydra head"},
    {id = 34696, buy = 0, sell = 370, subType = 0, name = "ice flower"},
    {id = 33315, buy = 0, sell = 720, subType = 0, name = "inkwell"},
    {id = 26172, buy = 0, sell = 300, subType = 0, name = "instable proto matter"},
    {id = 5880, buy = 0, sell = 500, subType = 0, name = "iron ore"},
    {id = 34582, buy = 0, sell = 180000, subType = 0, name = "izcandar's snow globe"},
    {id = 34583, buy = 0, sell = 225000, subType = 0, name = "izcandar's sundial"},
    {id = 12426, buy = 0, sell = 180, subType = 0, name = "jewelled belt"},
    {id = 15480, buy = 0, sell = 420, subType = 0, name = "kollos shell"},
    {id = 12427, buy = 0, sell = 100, subType = 0, name = "kongra's shoulderpad"},
    {id = 36276, buy = 0, sell = 330, subType = 0, name = "lamassu hoof"},
    {id = 36277, buy = 0, sell = 240, subType = 0, name = "lamassu horn"},
    {id = 11372, buy = 0, sell = 80, subType = 0, name = "lancer beetle shell"},
    {id = 11334, buy = 0, sell = 500, subType = 0, name = "legionnaire flags"},
    {id = 10608, buy = 0, sell = 60, subType = 0, name = "lion's mane"},
    {id = 12636, buy = 0, sell = 300, subType = 0, name = "lizard essence"},
    {id = 36175, buy = 0, sell = 530, subType = 0, name = "lizard heart"},
    {id = 5876, buy = 0, sell = 150, subType = 0, name = "lizard leather"},
    {id = 5881, buy = 0, sell = 120, subType = 0, name = "lizard scale"},
    {id = 30859, buy = 0, sell = 8000, subType = 0, name = "longing eyes"},
    {id = 12410, buy = 0, sell = 1000, subType = 0, name = "luminous orb"},
    {id = 10609, buy = 0, sell = 10, subType = 0, name = "lump of dirt"},
    {id = 11222, buy = 0, sell = 130, subType = 0, name = "lump of earth"},
    {id = 5904, buy = 0, sell = 8000, subType = 0, name = "magic sulphur"},
    {id = 34726, buy = 0, sell = 240000, subType = 0, name = "malofur's lunchbox"},
    {id = 11238, buy = 0, sell = 100, subType = 0, name = "mammoth tusk"},
    {id = 12445, buy = 0, sell = 280, subType = 0, name = "mantassin tail"},
    {id = 36275, buy = 0, sell = 310, subType = 0, name = "manticore ear"},
    {id = 36274, buy = 0, sell = 220, subType = 0, name = "manticore tail"},
    {id = 19741, buy = 0, sell = 65, subType = 0, name = "marsh stalker beak"},
    {id = 19742, buy = 0, sell = 50, subType = 0, name = "marsh stalker feather"},
    {id = 34580, buy = 0, sell = 500000, subType = 0, name = "maxxenius head"},
    {id = 36426, buy = 0, sell = 410000, subType = 0, name = "medal of valiance"},
    {id = 12428, buy = 0, sell = 75, subType = 0, name = "minotaur horn"},
    {id = 5878, buy = 0, sell = 80, subType = 0, name = "minotaur leather"},
    {id = 12430, buy = 0, sell = 60, subType = 0, name = "miraculum"},
    {id = 6537, buy = 0, sell = 50000, subType = 0, name = "mr. punish's handcuffs"},
    {id = 10579, buy = 0, sell = 420, subType = 0, name = "mutated bat ear"},
    {id = 11225, buy = 0, sell = 50, subType = 0, name = "mutated flesh"},
    {id = 10585, buy = 0, sell = 150, subType = 0, name = "mutated rat tail"},
    {id = 10577, buy = 0, sell = 700, subType = 0, name = "mystical hourglass"},
    {id = 12431, buy = 0, sell = 250, subType = 0, name = "necromantic robe"},
    {id = 11231, buy = 0, sell = 75, subType = 0, name = "nettle blossom"},
    {id = 12432, buy = 0, sell = 25, subType = 0, name = "nettle spit"},
    {id = 36430, buy = 0, sell = 430000, subType = 0, name = "noble amulet"},
    {id = 36428, buy = 0, sell = 425000, subType = 0, name = "noble cape"},
    {id = 12442, buy = 0, sell = 430, subType = 0, name = "noble turban"},
    {id = 5804, buy = 0, sell = 2000, subType = 0, name = "nose ring"},
    {id = 26166, buy = 0, sell = 410, subType = 0, name = "odd organ"},
    {id = 24844, buy = 0, sell = 180, subType = 0, name = "ogre ear stud"},
    {id = 12435, buy = 0, sell = 30, subType = 0, name = "orc leather"},
    {id = 11113, buy = 0, sell = 150, subType = 0, name = "orc tooth"},
    {id = 12433, buy = 0, sell = 85, subType = 0, name = "orcish gear"},
    {id = 32518, buy = 0, sell = 1350, subType = 0, name = "patch of fine cloth"},
    {id = 12437, buy = 0, sell = 30, subType = 0, name = "pelvis bone"},
    {id = 35045, buy = 0, sell = 200, subType = 0, name = "percht horns"},
    {id = 5893, buy = 0, sell = 250, subType = 0, name = "perfect behemoth fang"},
    {id = 11337, buy = 0, sell = 250, subType = 0, name = "petrified scream"},
    {id = 12439, buy = 0, sell = 20, subType = 0, name = "piece of archer armor"},
    {id = 11196, buy = 0, sell = 15, subType = 0, name = "piece of crocodile leather"},
    {id = 10580, buy = 0, sell = 420, subType = 0, name = "piece of dead brain"},
    {id = 6540, buy = 0, sell = 50000, subType = 0, name = "piece of massacre's shell"},
    {id = 10558, buy = 0, sell = 45, subType = 0, name = "piece of scarab shell"},
    {id = 12438, buy = 0, sell = 50, subType = 0, name = "piece of warrior armor"},
    {id = 10610, buy = 0, sell = 10, subType = 0, name = "pig foot"},
    {id = 12440, buy = 0, sell = 25, subType = 0, name = "pile of grave earth"},
    {id = 34725, buy = 0, sell = 280000, subType = 0, name = "plagueroot offshoot"},
    {id = 26162, buy = 0, sell = 250, subType = 0, name = "plasma pearls"},
    {id = 26176, buy = 0, sell = 270, subType = 0, name = "plasmatic lightning"},
    {id = 12441, buy = 0, sell = 10, subType = 0, name = "poison spider shell"},
    {id = 10557, buy = 0, sell = 50, subType = 0, name = "poisonous slime"},
    {id = 10567, buy = 0, sell = 30, subType = 0, name = "polar bear paw"},
    {id = 22541, buy = 0, sell = 480, subType = 0, name = "pool of chitinous glue"},
    {id = 27756, buy = 0, sell = 2000, subType = 0, name = "porcelain mask"},
    {id = 2803, buy = 0, sell = 10, subType = 0, name = "powder herb"},
    {id = 30853, buy = 0, sell = 15000, subType = 0, name = "pristine worm head"},
    {id = 12400, buy = 0, sell = 60, subType = 0, name = "protective charm"},
    {id = 12429, buy = 0, sell = 110, subType = 0, name = "purple robe"},
    {id = 12447, buy = 0, sell = 500, subType = 0, name = "quara bone"},
    {id = 12444, buy = 0, sell = 350, subType = 0, name = "quara eye"},
    {id = 12446, buy = 0, sell = 410, subType = 0, name = "quara pincers"},
    {id = 12443, buy = 0, sell = 140, subType = 0, name = "quara tentacle"},
    {id = 33314, buy = 0, sell = 1100, subType = 0, name = "quill"},
    {id = 30536, buy = 0, sell = 80, subType = 0, name = "rare earth"},
    {id = 5948, buy = 0, sell = 200, subType = 0, name = "red dragon leather"},
    {id = 5882, buy = 0, sell = 200, subType = 0, name = "red dragon scale"},
    {id = 36393, buy = 0, sell = 190, subType = 0, name = "red goanna scale"},
    {id = 5911, buy = 0, sell = 300, subType = 0, name = "red piece of cloth"},
    {id = 27056, buy = 0, sell = 175, subType = 0, name = "rhino hide"},
    {id = 27057, buy = 0, sell = 265, subType = 0, name = "rhino horn"},
    {id = 27054, buy = 0, sell = 300, subType = 0, name = "rhino horn carving"},
    {id = 12448, buy = 0, sell = 66, subType = 0, name = "rope belt"},
    {id = 36424, buy = 0, sell = 74000, subType = 0, name = "rotten heart"},
    {id = 11208, buy = 0, sell = 30, subType = 0, name = "rotten piece of cloth"},
    {id = 11228, buy = 0, sell = 400, subType = 0, name = "sabretooth"},
    {id = 12449, buy = 0, sell = 120, subType = 0, name = "safety pin"},
    {id = 32648, buy = 0, sell = 250, subType = 0, name = "sample of monster blood"},
    {id = 11373, buy = 0, sell = 20, subType = 0, name = "sandcrawler shell"},
    {id = 27607, buy = 0, sell = 10000, subType = 0, name = "scale of gelidrazah"},
    {id = 12629, buy = 0, sell = 680, subType = 0, name = "scale of corruption"},
    {id = 10548, buy = 0, sell = 280, subType = 0, name = "scarab pincers"},
    {id = 10568, buy = 0, sell = 25, subType = 0, name = "scorpion tail"},
    {id = 12466, buy = 0, sell = 230, subType = 0, name = "scroll of heroic deeds"},
    {id = 11229, buy = 0, sell = 450, subType = 0, name = "scythe leg"},
    {id = 36158, buy = 0, sell = 42000, subType = 0, name = "sea horse figurine"},
    {id = 10583, buy = 0, sell = 520, subType = 0, name = "sea serpent scale"},
    {id = 7732, buy = 0, sell = 150, subType = 0, name = "seeds"},
    {id = 11324, buy = 0, sell = 25, subType = 0, name = "shaggy tail"},
    {id = 12434, buy = 0, sell = 45, subType = 0, name = "shamanic hood"},
    {id = 24840, buy = 0, sell = 200, subType = 0, name = "shamanic talisman"},
    {id = 28993, buy = 0, sell = 150, subType = 0, name = "shimmering beetles"},
    {id = 22517, buy = 0, sell = 3000, subType = 0, name = "sight of surrender's eye"},
    {id = 36427, buy = 0, sell = 480000, subType = 0, name = "signet ring"},
    {id = 22535, buy = 0, sell = 600, subType = 0, name = "silencer resonating chamber"},
    {id = 33313, buy = 0, sell = 550, subType = 0, name = "silken bookmark"},
    {id = 11209, buy = 0, sell = 35, subType = 0, name = "silky fur"},
    {id = 6526, buy = 0, sell = 3000, subType = 0, name = "skeleton decoration"},
    {id = 12436, buy = 0, sell = 80, subType = 0, name = "skull belt"},
    {id = 24847, buy = 0, sell = 250, subType = 0, name = "skull fetish"},
    {id = 11191, buy = 0, sell = 50, subType = 0, name = "skunk tail"},
    {id = 30858, buy = 0, sell = 4500, subType = 0, name = "slimy leg"},
    {id = 26180, buy = 0, sell = 250, subType = 0, name = "small energy ball"},
    {id = 12468, buy = 0, sell = 95, subType = 0, name = "small flask of eyedrops"},
    {id = 12406, buy = 0, sell = 480, subType = 0, name = "small notebook"},
    {id = 2063, buy = 0, sell = 150, subType = 0, name = "small oil lamp"},
    {id = 12469, buy = 0, sell = 70, subType = 0, name = "small pitchfork"},
    {id = 10611, buy = 0, sell = 400, subType = 0, name = "snake skin"},
    {id = 5875, buy = 0, sell = 2000, subType = 0, name = "sniper gloves"},
    {id = 26173, buy = 0, sell = 310, subType = 0, name = "solid rage"},
    {id = 5809, buy = 0, sell = 6000, subType = 0, name = "soul stone"},
    {id = 26174, buy = 0, sell = 350, subType = 0, name = "spark sphere"},
    {id = 26158, buy = 0, sell = 290, subType = 0, name = "sparkion claw"},
    {id = 26160, buy = 0, sell = 310, subType = 0, name = "sparkion legs"},
    {id = 26161, buy = 0, sell = 280, subType = 0, name = "sparkion stings"},
    {id = 26159, buy = 0, sell = 300, subType = 0, name = "sparkion tail"},
    {id = 36272, buy = 0, sell = 470, subType = 0, name = "sphinx feather"},
    {id = 36273, buy = 0, sell = 360, subType = 0, name = "sphinx tiara"},
    {id = 8859, buy = 0, sell = 10, subType = 0, name = "spider fangs"},
    {id = 5879, buy = 0, sell = 100, subType = 0, name = "spider silk"},
    {id = 11325, buy = 0, sell = 100, subType = 0, name = "spiked iron ball"},
    {id = 5884, buy = 0, sell = 40000, subType = 0, name = "spirit container"},
    {id = 10559, buy = 0, sell = 95, subType = 0, name = "spooky blue eye"},
    {id = 2800, buy = 0, sell = 15, subType = 0, name = "star herb"},
    {id = 2799, buy = 0, sell = 20, subType = 0, name = "stone herb"},
    {id = 30841, buy = 0, sell = 100, subType = 0, name = "stonerefiner's skull"},
    {id = 11195, buy = 0, sell = 120, subType = 0, name = "stone wing"},
    {id = 11226, buy = 0, sell = 600, subType = 0, name = "strand of medusa hair"},
    {id = 26169, buy = 0, sell = 300, subType = 0, name = "strange proto matter"},
    {id = 2174, buy = 0, sell = 200, subType = 0, name = "strange symbol"},
    {id = 11210, buy = 0, sell = 50, subType = 0, name = "striped fur"},
    {id = 10603, buy = 0, sell = 20, subType = 0, name = "swamp grass"},
    {id = 12628, buy = 0, sell = 240, subType = 0, name = "tail of corruption"},
    {id = 11198, buy = 0, sell = 80, subType = 0, name = "tarantula egg"},
    {id = 27055, buy = 0, sell = 320, subType = 0, name = "tarnished rhino figurine"},
    {id = 10601, buy = 0, sell = 120, subType = 0, name = "tattered piece of robe"},
    {id = 12622, buy = 0, sell = 5000, subType = 0, name = "tentacle piece"},
    {id = 11370, buy = 0, sell = 50, subType = 0, name = "terramite eggs"},
    {id = 11371, buy = 0, sell = 60, subType = 0, name = "terramite legs"},
    {id = 11369, buy = 0, sell = 170, subType = 0, name = "terramite shell"},
    {id = 11190, buy = 0, sell = 95, subType = 0, name = "terrorbird beak"},
    {id = 6539, buy = 0, sell = 50000, subType = 0, name = "the handmaiden's protector"},
    {id = 6534, buy = 0, sell = 50000, subType = 0, name = "the imperor's trident"},
    {id = 6535, buy = 0, sell = 50000, subType = 0, name = "the plasmother's remains"},
    {id = 11224, buy = 0, sell = 150, subType = 0, name = "thick fur"},
    {id = 10560, buy = 0, sell = 100, subType = 0, name = "thorn"},
    {id = 36429, buy = 0, sell = 440000, subType = 0, name = "token of love"},
    {id = 27608, buy = 0, sell = 10000, subType = 0, name = "tooth of tazhadur"},
    {id = 29040, buy = 0, sell = 250, subType = 0, name = "torn shirt"},
    {id = 22537, buy = 0, sell = 900, subType = 0, name = "trapped bad dream monster"},
    {id = 2805, buy = 0, sell = 25, subType = 0, name = "troll green"},
    {id = 12471, buy = 0, sell = 50, subType = 0, name = "trollroot"},
    {id = 30830, buy = 0, sell = 500, subType = 0, name = "tunnel tyrant head"},
    {id = 30831, buy = 0, sell = 700, subType = 0, name = "tunnel tyrant shell"},
    {id = 5899, buy = 0, sell = 90, subType = 0, name = "turtle shell"},
    {id = 3956, buy = 0, sell = 100, subType = 0, name = "tusk"},
    {id = 36458, buy = 0, sell = 490000, subType = 0, name = "urmahlullus mane"},
    {id = 36459, buy = 0, sell = 245000, subType = 0, name = "urmahlullus paws"},
    {id = 36457, buy = 0, sell = 210000, subType = 0, name = "urmahlullus tail"},
    {id = 11367, buy = 0, sell = 200, subType = 0, name = "undead heart"},
    {id = 11233, buy = 0, sell = 480, subType = 0, name = "unholy bone"},
    {id = 5905, buy = 0, sell = 100, subType = 0, name = "vampire dust"},
    {id = 10602, buy = 0, sell = 275, subType = 0, name = "vampire teeth"},
    {id = 33985, buy = 0, sell = 2150, subType = 0, name = "violet glass plate"},
    {id = 26170, buy = 0, sell = 300, subType = 0, name = "volatile proto matter"},
    {id = 11322, buy = 0, sell = 200, subType = 0, name = "warmaster's wristguards"},
    {id = 11235, buy = 0, sell = 30, subType = 0, name = "warwolf fur"},
    {id = 11314, buy = 0, sell = 250, subType = 0, name = "weaver's wandtip"},
    {id = 11234, buy = 0, sell = 380, subType = 0, name = "werewolf fur"},
    {id = 5909, buy = 0, sell = 100, subType = 0, name = "white piece of cloth"},
    {id = 11328, buy = 0, sell = 110, subType = 0, name = "widow's mandibles"},
    {id = 28991, buy = 0, sell = 120, subType = 0, name = "wild flowers"},
    {id = 11230, buy = 0, sell = 800, subType = 0, name = "winged tail"},
    {id = 11212, buy = 0, sell = 20, subType = 0, name = "winter wolf fur"},
    {id = 30842, buy = 0, sell = 850, subType = 0, name = "withered pauldrons"},
    {id = 30843, buy = 0, sell = 900, subType = 0, name = "withered scalp"},
    {id = 10569, buy = 0, sell = 60, subType = 0, name = "witch broom"},
    {id = 5897, buy = 0, sell = 70, subType = 0, name = "wolf paw"},
    {id = 5901, buy = 0, sell = 5, subType = 0, name = "wood"},
    {id = 11236, buy = 0, sell = 15, subType = 0, name = "wool"},
    {id = 10582, buy = 0, sell = 400, subType = 0, name = "wyrm scale"},
    {id = 10561, buy = 0, sell = 265, subType = 0, name = "wyvern talisman"},
    {id = 5914, buy = 0, sell = 150, subType = 0, name = "yellow piece of cloth"},
    {id = 36425, buy = 0, sell = 25000, subType = 0, name = "young lich worm"},
    {id = 11330, buy = 0, sell = 600, subType = 0, name = "zaogun flag"},
    {id = 11331, buy = 0, sell = 150, subType = 0, name = "zaogun shoulderplates"},
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

npcType:eventType(NPCS_EVENT_SELLITEM)
npcType:onSellItem(function(npc, player, itemId, subType, amount, ignoreEquipped)
    local shopItem = getShopItem(itemId, subType, false)
    if not shopItem or shopItem.sell <= 0 then return false end
    local totalPrice = amount * shopItem.sell
    local itemName = shopItem.name or ItemType(itemId):getName()
    
    local itemSubType = -1
    if ItemType(itemId):isFluidContainer() then
        itemSubType = subType
    end
    
    if doPlayerSellItem(player:getId(), itemId, amount, totalPrice, itemSubType, ignoreEquipped) then
        player:sendTextMessage(MESSAGE_INFO_DESCR, "Sold " .. amount .. "x " .. shopItem.name .. " for " .. (amount * shopItem.sell) .. " gold.")
        return true
    end
    player:sendCancelMessage("You do not have this object.")
    return false
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
