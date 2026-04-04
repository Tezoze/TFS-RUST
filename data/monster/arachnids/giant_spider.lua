local mType = Game.createMonsterType("Giant Spider")
local monster = {}

monster.description = "a giant spider"
monster.experience = 900
monster.outfit = {
	lookType = 38,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5977
monster.health = 1300
monster.maxHealth = 1300
monster.race = "venom"
monster.speed = 230
monster.manaCost = 0
monster.maxSummons = 2

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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 70,
	runHealth = 0,
	canWalkOnFire = false,
	canWalkOnEnergy = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 195}, -- gold coin
	{id = 2545, chance = 12500, maxCount = 12}, -- poison arrow
	{id = 2463, chance = 10000}, -- plate armor
	{id = 2647, chance = 8000}, -- plate legs
	{id = 2377, chance = 5000}, -- two handed sword
	{id = 2457, chance = 5000}, -- steel helmet
	{id = 7588, chance = 3571}, -- strong health potion
	{id = 5879, chance = 2000}, -- spider silk
	{id = 2477, chance = 870}, -- knight legs
	{id = 2169, chance = 710}, -- time ring
	{id = 2476, chance = 500}, -- knight armor
	{id = 2171, chance = 280}, -- platinum amulet
	{id = 7901, chance = 270}, -- lightning headband
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -300, target = false, condition = {type = CONDITION_POISON, startDamage = 160, interval = 2000}},
	{name = "poisonfield", interval = 2000, chance = 10, range = 7, radius = 1, shootEffect = CONST_ANI_POISON, target = true},
	{name = "combat", interval = 2000, chance = 10, minDamage = -40, maxDamage = -70, range = 7, radius = 1, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 390, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}

monster.summons = {
	{name = "Poison Spider", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)