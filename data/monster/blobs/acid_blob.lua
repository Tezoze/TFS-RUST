local mType = Game.createMonsterType("Acid Blob")
local monster = {}

monster.description = "an acid blob"
monster.experience = 250
monster.outfit = {
	lookType = 314,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 9962
monster.health = 250
monster.maxHealth = 250
monster.race = "venom"
monster.speed = 120
monster.manaCost = 0
monster.maxSummons = 3

monster.changeTarget = {
	interval = 5000,
	chance = 0
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
	staticAttackChance = 85,
	runHealth = 0,
	canWalkOnPoison = true,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Kzzchhhh", yell = false},
}

monster.loot = {
	{id = 9967, chance = 18520}, -- glob of acid slime
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -80, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = -10, maxDamage = -20, radius = 4, effect = CONST_ME_GREENSPARK, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -40, maxDamage = -60, effect = CONST_ME_GREENBUBBLE, target = false, length = 5, spread = 0, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 1,
	armor = 3,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}

monster.summons = {
	{name = "Acid Blob", chance = 10, interval = 2000, max = 3},
}

mType:register(monster)