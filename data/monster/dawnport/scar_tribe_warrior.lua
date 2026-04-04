local mType = Game.createMonsterType("Orc Warrior")
local monster = {}

monster.description = "an orc warrior"
monster.experience = 50
monster.outfit = {
	lookType = 7,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5979
monster.health = 125
monster.maxHealth = 125
monster.race = "blood"
monster.speed = 190
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Alk!", yell = false},
	{text = "Trak grrrr brik.", yell = false},
	{text = "Grow truk grrrr.", yell = false},
}

monster.loot = {
	{id = 12409, chance = 6740}, -- broken helmet
	{id = 2464, chance = 5620}, -- chain armor
	{id = 2148, chance = 100000, maxCount = 8}, -- gold coin
	{id = 2666, chance = 13480}, -- meat
	{id = 12435, chance = 5620}, -- orc leather
	{id = 12436, chance = 1120}, -- skull belt
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 10, attack = 25, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 8,
}


mType:register(monster)