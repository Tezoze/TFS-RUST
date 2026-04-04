local mType = Game.createMonsterType("Deadeye Devious")
local monster = {}

monster.description = "Deadeye Devious"
monster.experience = 750
monster.outfit = {
	lookType = 151,
	lookHead = 115,
	lookBody = 76,
	lookLegs = 35,
	lookFeet = 117,
	lookAddons = 2,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 1450
monster.maxHealth = 1450
monster.race = "blood"
monster.speed = 300
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 60000,
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
	canPushCreatures = true,
	targetDistance = 3,
	staticAttackChance = 50,
	runHealth = 150,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Let's kill 'em", yell = false},
	{text = "Arrrgh!", yell = false},
	{text = "You'll never take me alive!", yell = false},
	{text = "You won't get me alive!", yell = false},
	{text = "§%§&§! #*$§$!!", yell = false},
}

monster.loot = {
	{id = 6102, chance = 100000}, -- deadeye devious' eye patch
	{id = 2148, chance = 100000, maxCount = 140}, -- gold coin
	{id = 2229, chance = 85000, maxCount = 2}, -- skull
	{id = 2463, chance = 78000}, -- plate armor
	{id = 2666, chance = 42000, maxCount = 3}, -- meat
	{id = 2476, chance = 28000}, -- knight armor
	{id = 2379, chance = 21000}, -- dagger
	{id = 2145, chance = 14000}, -- small diamond
	{id = 2387, chance = 7000}, -- double axe
	{id = 5926, chance = 7000}, -- pirate backpack
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -150, target = false},
	{name = "combat", interval = 4000, chance = 60, minDamage = 0, maxDamage = -350, range = 7, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 50,
	armor = 35,
	{name = "combat", interval = 4000, chance = 25, minDamage = 50, maxDamage = 150, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)