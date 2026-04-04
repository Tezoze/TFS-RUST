local mType = Game.createMonsterType("The Mutated Pumpkin")
local monster = {}

monster.description = "The Mutated Pumpkin"
monster.experience = 30000
monster.outfit = {
	lookType = 292,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8960
monster.health = 500000
monster.maxHealth = 500000
monster.race = "undead"
monster.speed = 400
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	targetDistance = 1,
	staticAttackChance = 85,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 20,
	{text = "I had the Halloween Hare for breakfast!", yell = false},
	{text = "Your soul will be mine...wait, wrong line", yell = false},
	{text = "Trick or treat? I saw death!", yell = false},
	{text = "No wait! Don't kill me! It's me, your friend!", yell = false},
	{text = "Bunnies, bah! I'm the real thing!", yell = false},
	{text = "Muahahahaha!", yell = false},
	{text = "I've come to avenge all those mutilated pumpkins!", yell = false},
	{text = "Wait until I get you!", yell = false},
	{text = "Fear the spirit of Halloween!", yell = false},
}

monster.loot = {
	{id = 2683, chance = 100000}, -- pumpkin
	{id = 9005, chance = 100000, maxCount = 20}, -- yummy gummy worm
	{id = 2688, chance = 1000, maxCount = 50}, -- candy cane
	{id = 6569, chance = 1000, maxCount = 50}, -- candy
	{id = 8860, chance = 1000}, -- spiderwebs
	{id = 9006, chance = 1000}, -- toy spider
	{id = 6492, chance = 1000}, -- bat decoration
	{id = 6526, chance = 1000}, -- skeleton decoration
	{id = 6574, chance = 1000}, -- bar of chocolate
	{id = 6570, chance = 1000},
	{id = 6571, chance = 1000},
	{id = 2096, chance = 1000}, -- pumpkinhead
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 105, attack = 85, target = false},
	{name = "combat", interval = 3000, chance = 18, minDamage = -100, maxDamage = -300, range = 7, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 15, minDamage = -100, maxDamage = -300, radius = 7, effect = CONST_ME_POISON, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 3000, chance = 14, minDamage = -40, maxDamage = -300, radius = 7, effect = CONST_ME_ENERGY, target = false, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 1000, chance = 10, minDamage = -30, maxDamage = -300, radius = 8, effect = CONST_ME_POFF, target = false, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 3000, chance = 12, minDamage = -100, maxDamage = -300, effect = CONST_ME_PLANTATTACK, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 1000, chance = 10, minDamage = -100, maxDamage = -400, effect = CONST_ME_GREENSPARK, target = false, length = 6, spread = 0, type = COMBAT_LIFEDRAIN},
	{name = "outfit", interval = 1000, chance = 2, radius = 8, effect = CONST_ME_BLUEBUBBLE, target = false},
}

monster.defenses = {
	defense = 60,
	armor = 60,
	{name = "combat", interval = 4000, chance = 15, minDamage = 2000, maxDamage = 2900, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)