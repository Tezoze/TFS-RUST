local mType = Game.createMonsterType("Witch")
local monster = {}

monster.description = "a witch"
monster.experience = 120
monster.outfit = {
	lookType = 54,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 300
monster.maxHealth = 300
monster.race = "blood"
monster.speed = 204
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
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 30,
	canWalkOnFire = false,
	canWalkOnPoison = false,
	canWalkOnEnergy = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Horax Pokti!", yell = false},
	{text = "Herba budinia ex!", yell = false},
	{text = "Hihihihi!", yell = false},
}

monster.loot = {
	{id = 2129, chance = 10120}, -- wolf tooth chain
	{id = 2148, chance = 64000, maxCount = 40}, -- gold coin
	{id = 2185, chance = 1140}, -- necrotic rod
	{id = 2199, chance = 1000}, -- garlic necklace
	{id = 2402, chance = 500}, -- silver dagger
	{id = 2405, chance = 3910}, -- sickle
	{id = 2643, chance = 4950}, -- leather boots
	{id = 2651, chance = 2010}, -- coat
	{id = 2654, chance = 4870}, -- cape
	{id = 2687, chance = 29750, maxCount = 8}, -- cookie
	{id = 2800, chance = 8950}, -- star herb
	{id = 10569, chance = 10000}, -- witch broom
	{id = 10570, chance = 80}, -- witch hat
	{id = 11211, chance = 10}, -- stuffed toad
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -20, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -30, maxDamage = -75, range = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIRE, target = true, type = COMBAT_FIREDAMAGE},
	{name = "firefield", interval = 2000, chance = 10, range = 7, radius = 1, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "outfit", interval = 2000, chance = 1, range = 5, target = true},
}

monster.defenses = {
	defense = 15,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "energy", combat = true, condition = true},
}


mType:register(monster)