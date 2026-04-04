local mType = Game.createMonsterType("Squidgy Slime")
local monster = {}

monster.description = "a slime"
monster.experience = 55
monster.outfit = {
	lookType = 19,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8062
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Blubb", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 45, attack = 40, target = false},
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
	{name = "Squidgy Slime", chance = 10, interval = 2000, max = 3},
}

mType:register(monster)