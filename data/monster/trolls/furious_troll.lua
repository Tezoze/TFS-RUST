local mType = Game.createMonsterType("Furious Troll")
local monster = {}

monster.description = "a furious troll"
monster.experience = 185
monster.outfit = {
	lookType = 15,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5960
monster.health = 245
monster.maxHealth = 245
monster.race = "blood"
monster.speed = 180
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 2000,
	chance = 5
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Slice! Slice!", yell = false},
	{text = "DIE!!!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 93000, maxCount = 146}, -- gold coin
	{id = 2152, chance = 6000}, -- platinum coin
	{id = 2391, chance = 750}, -- war hammer
	{id = 10606, chance = 4400}, -- bunch of troll hair
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 15,
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Mechanical Fighter", chance = 90, interval = 2000, max = 2},
}

mType:register(monster)