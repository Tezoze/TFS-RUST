local mType = Game.createMonsterType("Deepsea Blood Crab")
local monster = {}

monster.description = "a deepsea blood crab"
monster.experience = 180
monster.outfit = {
	lookType = 200,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6075
monster.health = 320
monster.maxHealth = 320
monster.race = "blood"
monster.speed = 380
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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 88000, maxCount = 20}, -- gold coin
	{id = 2667, chance = 9800}, -- fish
	{id = 10550, chance = 7220}, -- bloody pincers
	{id = 2464, chance = 4330}, -- chain armor
	{id = 2478, chance = 2820}, -- brass legs
	{id = 2143, chance = 340}, -- white pearl
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -111, target = false},
}

monster.defenses = {
	defense = 28,
	armor = 28,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)