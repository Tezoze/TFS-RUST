local mType = Game.createMonsterType("Marid")
local monster = {}

monster.description = "a marid"
monster.experience = 410
monster.outfit = {
	lookType = 104,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6033
monster.health = 550
monster.maxHealth = 550
monster.race = "blood"
monster.speed = 234
monster.manaCost = 0
monster.maxSummons = 2

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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Simsalabim", yell = false},
	{text = "Feel the power of my magic, tiny mortal!", yell = false},
	{text = "Wishes can come true", yell = false},
	{text = "Be careful what you wish.", yell = false},
	{text = "Djinns will soon again be the greatest!", yell = false},
}

monster.loot = {
	{id = 1872, chance = 2560}, -- blue tapestry
	{id = 2063, chance = 110}, -- small oil lamp
	{id = 2146, chance = 6200}, -- small sapphire
	{id = 2148, chance = 60000, maxCount = 70}, -- gold coin
	{id = 2148, chance = 60000, maxCount = 30}, -- gold coin
	{id = 2158, chance = 110}, -- blue gem
	{id = 2183, chance = 770}, -- hailstorm rod
	{id = 2374, chance = 5000}, -- wooden flute
	{id = 2442, chance = 4530}, -- heavy machete
	{id = 2663, chance = 290}, -- mystic turban
	{id = 2677, chance = 65000, maxCount = 29}, -- blueberry
	{id = 5912, chance = 3750}, -- blue piece of cloth
	{id = 7378, chance = 15500, maxCount = 3}, -- royal spear
	{id = 7589, chance = 9800}, -- strong mana potion
	{id = 7732, chance = 2400}, -- seeds
	{id = 7900, chance = 320}, -- magma monocle
	{id = 12426, chance = 7880}, -- jewelled belt
	{id = 12442, chance = 530}, -- noble turban
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -90, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -250, range = 7, shootEffect = CONST_ANI_ENERGYBALL, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -90, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 2000, chance = 15, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -650, duration = 1500},
	{name = "drunk", interval = 2000, chance = 10, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, duration = 6000},
	{name = "outfit", interval = 2000, chance = 1, range = 7, effect = CONST_ME_BLUESHIMMER, target = false},
	{name = "combat", interval = 2000, chance = 15, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -90, radius = 3, effect = CONST_ME_ENERGY, target = false, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 20,
	armor = 24,
	{name = "combat", interval = 2000, chance = 15, minDamage = 50, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 90},
	{type = COMBAT_EARTHDAMAGE, percent = 1},
	{type = COMBAT_ENERGYDAMAGE, percent = 60},
	{type = COMBAT_HOLYDAMAGE, percent = 1},
	{type = COMBAT_ICEDAMAGE, percent = -1},
	{type = COMBAT_DEATHDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "blue djinn", chance = 10, interval = 2000, max = 2},
}

mType:register(monster)