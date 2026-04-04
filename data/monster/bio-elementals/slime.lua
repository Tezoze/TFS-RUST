local mType = Game.createMonsterType("Slime")
local monster = {}

monster.description = "a slime"
monster.experience = 160
monster.outfit = {
	lookType = 19,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 1490
monster.health = 150
monster.maxHealth = 150
monster.race = "venom"
monster.speed = 120
monster.manaCost = 0
monster.maxSummons = 3

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
	{text = "Blubb", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -110, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 3,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
}

monster.summons = {
	{name = "Slime", chance = 10, interval = 2000, max = 3},
}

mType:register(monster)