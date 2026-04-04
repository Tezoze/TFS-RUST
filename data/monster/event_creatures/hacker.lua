local mType = Game.createMonsterType("Hacker")
local monster = {}

monster.description = "a hacker"
monster.experience = 45
monster.outfit = {
	lookType = 8,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5980
monster.health = 430
monster.maxHealth = 430
monster.race = "blood"
monster.speed = 250
monster.manaCost = 350
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 429,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Feel the wrath of me dos attack!", yell = false},
	{text = "You're next!", yell = false},
	{text = "Gimme free gold!", yell = false},
	{text = "Me sooo smart!", yell = false},
	{text = "Me have a cheating link for you!", yell = false},
	{text = "Me is GM!", yell = false},
	{text = "Gimme your password!", yell = false},
	{text = "Me just need the code!", yell = false},
	{text = "Me not stink!", yell = false},
	{text = "Me other char is highlevel!", yell = false},
}

monster.loot = {
	{id = 2044, chance = 6666}, -- lamp
	{id = 2148, chance = 100000, maxCount = 12}, -- gold coin
	{id = 2378, chance = 5000}, -- battle axe
	{id = 2381, chance = 10000}, -- halberd
	{id = 2386, chance = 10000}, -- axe
	{id = 2391, chance = 5000}, -- war hammer
	{id = 2671, chance = 50000}, -- ham
	{id = 6570, chance = 5538},
	{id = 6571, chance = 1538},
}

monster.attacks = {
	{name = "melee", interval = 1000, minDamage = 0, maxDamage = -83, target = false},
}

monster.defenses = {
	defense = 12,
	armor = 15,
	{name = "speed", interval = 1000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 290, duration = 6000},
	{name = "outfit", interval = 10000, chance = 15, effect = CONST_ME_REDSHIMMER, monster = "pig", duration = 500},
}

monster.immunities = {
	{type = "energy", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drunk", combat = false, condition = true},
}


mType:register(monster)