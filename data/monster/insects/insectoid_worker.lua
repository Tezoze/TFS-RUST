local mType = Game.createMonsterType("Insectoid Worker")
local monster = {}

monster.description = "an insectoid worker"
monster.experience = 650
monster.outfit = {
	lookType = 403,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 13514
monster.health = 950
monster.maxHealth = 950
monster.race = "venom"
monster.speed = 200
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
	illusionable = true,
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

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 90}, -- gold coin
	{id = 2149, chance = 2880}, -- small emerald
	{id = 2438, chance = 560}, -- epee
	{id = 7618, chance = 5090}, -- health potion
	{id = 15486, chance = 15380}, -- compound eye
	{id = 15622, chance = 14990}, -- dung ball
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -163, target = false, condition = {type = CONDITION_POISON, startDamage = 160, interval = 4000}},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
