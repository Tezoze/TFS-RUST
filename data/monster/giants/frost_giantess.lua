local mType = Game.createMonsterType("Frost Giantess")
local monster = {}

monster.description = "a frost giantess"
monster.experience = 150
monster.outfit = {
	lookType = 265,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7330
monster.health = 275
monster.maxHealth = 275
monster.race = "blood"
monster.speed = 194
monster.manaCost = 490
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
	convinceable = true,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 60,
	targetDistance = 4,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ymirs Mjalle!", yell = false},
	{text = "No run so much, must stay fat!", yell = false},
	{text = "Hörre Sjan Flan!", yell = false},
	{text = "Damned fast food.", yell = false},
	{text = "Come kiss the cook!", yell = false},
}

monster.loot = {
	{id = 1294, chance = 10360, maxCount = 3}, -- small stone
	{id = 2148, chance = 80000, maxCount = 40}, -- gold coin
	{id = 2207, chance = 70}, -- melee ring
	{id = 2406, chance = 7960}, -- short sword
	{id = 2490, chance = 170}, -- dark helmet
	{id = 2513, chance = 1490}, -- battle shield
	{id = 2671, chance = 20990, maxCount = 2}, -- ham
	{id = 7290, chance = 1000}, -- shard
	{id = 7441, chance = 2008}, -- ice cube
	{id = 7460, chance = 320}, -- norse shield
	{id = 7620, chance = 950}, -- mana potion
	{id = 10575, chance = 4800}, -- frost giant pelt
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -60, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = 0, maxDamage = -90, range = 7, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 300, duration = 5000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -3},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)