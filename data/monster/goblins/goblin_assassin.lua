local mType = Game.createMonsterType("Goblin Assassin")
local monster = {}

monster.description = "a goblin assassin"
monster.experience = 52
monster.outfit = {
	lookType = 296,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6002
monster.health = 75
monster.maxHealth = 75
monster.race = "blood"
monster.speed = 140
monster.manaCost = 360
monster.maxSummons = 0

monster.changeTarget = {
	interval = 10000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Goblin Powahhh!", yell = false},
	{text = "Me kill you!", yell = false},
	{text = "Me green menace!", yell = false},
	{text = "Gobabunga!", yell = false},
	{text = "Gooobliiiins!", yell = false},
}

monster.loot = {
	{id = 1294, chance = 9900, maxCount = 3}, -- small stone
	{id = 2148, chance = 50000, maxCount = 9}, -- gold coin
	{id = 2230, chance = 13000}, -- bone
	{id = 2235, chance = 6610}, -- mouldy cheese
	{id = 2379, chance = 17000}, -- dagger
	{id = 2406, chance = 8820}, -- short sword
	{id = 2449, chance = 4770}, -- bone club
	{id = 2461, chance = 13000}, -- leather helmet
	{id = 2467, chance = 7240}, -- leather armor
	{id = 2559, chance = 9800}, -- small axe
	{id = 2667, chance = 12400}, -- fish
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -15, target = false},
	{name = "combat", interval = 2000, chance = 10, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -35, range = 6, shootEffect = CONST_ANI_THROWINGKNIFE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 6,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 100, duration = 3000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = 1},
	{type = COMBAT_DEATHDAMAGE, percent = -1},
}


mType:register(monster)