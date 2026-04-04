local mType = Game.createMonsterType("Minishabaal")
local monster = {}

monster.description = "Minishabaal"
monster.experience = 4000
monster.outfit = {
	lookType = 237,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6364
monster.health = 3500
monster.maxHealth = 3500
monster.race = "blood"
monster.speed = 700
monster.manaCost = 0
monster.maxSummons = 3

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
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 350,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I had Princess Lumelia as breakfast!", yell = false},
	{text = "Naaa-Nana-Naaa-Naaa!", yell = false},
	{text = "My brother will come and get you for this!", yell = false},
	{text = "Get them Fluffy!", yell = false},
	{text = "He He He!", yell = false},
	{text = "Pftt, Ferumbras such an upstart!", yell = false},
	{text = "My dragon is not that old, it's just second hand!", yell = false},
	{text = "My other dragon is a red one!", yell = false},
	{text = "When I am big I want to become the ruthless eighth!", yell = false},
	{text = "WHERE'S FLUFFY?", yell = false},
	{text = "Muahaha!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 20}, -- gold coin
	{id = 2150, chance = 1428, maxCount = 2}, -- small amethyst
	{id = 2548, chance = 2857}, -- pitchfork
	{id = 2432, chance = 666}, -- fire axe
	{id = 5944, chance = 909}, -- soul orb
	{id = 2520, chance = 200}, -- demon shield
	{id = 6500, chance = 1000, maxCount = 2}, -- demonic essence
	{id = 2470, chance = 180}, -- golden legs
	{id = 2148, chance = 100000, maxCount = 20}, -- gold coin
	{id = 5944, chance = 909}, -- soul orb
	{id = 2488, chance = 800}, -- crown legs
	{id = 2515, chance = 1333}, -- guardian shield
	{id = 2136, chance = 909}, -- demonbone amulet
	{id = 2542, chance = 500}, -- tempest shield
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 70, attack = 95, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -80, maxDamage = -350, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 3000, chance = 34, minDamage = -120, maxDamage = -500, range = 7, radius = 2, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "combat", interval = 1000, chance = 50, minDamage = 155, maxDamage = 255, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 1000, chance = 12, effect = CONST_ME_REDSHIMMER, speed = 320, duration = 4000},
	{name = "invisible", interval = 4000, chance = 50, effect = CONST_ME_REDSHIMMER},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Diabolic Imp", chance = 40, interval = 2000, max = 3},
}

mType:register(monster)