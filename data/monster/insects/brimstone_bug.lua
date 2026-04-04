local mType = Game.createMonsterType("Brimstone Bug")
local monster = {}

monster.description = ""
monster.experience = 900
monster.outfit = {
	lookType = 352,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 12527
monster.health = 1300
monster.maxHealth = 1300
monster.race = "venom"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Chrrr!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2149, chance = 2702, maxCount = 4}, -- small emerald
	{id = 2165, chance = 892}, -- stealth ring
	{id = 2171, chance = 110}, -- platinum amulet
	{id = 5904, chance = 1639}, -- magic sulphur
	{id = 7588, chance = 9003}, -- strong health potion
	{id = 7589, chance = 9025}, -- strong mana potion
	{id = 10557, chance = 50000}, -- poisonous slime
	{id = 11222, chance = 20000}, -- lump of earth
	{id = 11232, chance = 14970}, -- sulphurous stone
	{id = 12658, chance = 5710}, -- brimstone fangs
	{id = 12659, chance = 10000}, -- brimstone shell
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -213, target = false, condition = {type = CONDITION_POISON, startDamage = 400, interval = 2000}},
	{name = "speed", interval = 2000, chance = 20, range = 7, shootEffect = CONST_ANI_DEATH, effect = CONST_ME_MORTAREA, target = true, speed = -600, duration = 10000},
	{name = "combat", interval = 2000, chance = 5, minDamage = -140, maxDamage = -310, radius = 6, effect = CONST_ME_SMALLPLANTS, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -130, maxDamage = -200, effect = CONST_ME_GREENSPARK, target = false, length = 6, spread = 0, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 15, minDamage = -80, maxDamage = -120, effect = CONST_ME_YELLOWBUBBLE, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)