local mType = Game.createMonsterType("Terramite")
local monster = {}

monster.description = "a terramite"
monster.experience = 160
monster.outfit = {
	lookType = 346,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11347
monster.health = 365
monster.maxHealth = 365
monster.race = "venom"
monster.speed = 222
monster.manaCost = 505
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Zrp zrp!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 97520, maxCount = 45}, -- gold coin
	{id = 11369, chance = 7730}, -- terramite shell
	{id = 11370, chance = 4680, maxCount = 3}, -- terramite eggs
	{id = 11371, chance = 14880}, -- terramite legs
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -5, maxDamage = -16, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)