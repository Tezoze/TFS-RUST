local mType = Game.createMonsterType("General Murius")
local monster = {}

monster.description = "General Murius"
monster.experience = 450
monster.outfit = {
	lookType = 29,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5983
monster.health = 550
monster.maxHealth = 550
monster.race = "blood"
monster.speed = 250
monster.manaCost = 0
monster.maxSummons = 4

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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "You will get what you deserve!", yell = false},
	{text = "Feel the power of the Mooh'Tah!", yell = false},
	{text = "For the king!", yell = false},
	{text = "Guards!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 90}, -- gold coin
	{id = 12428, chance = 100000}, -- minotaur horn
	{id = 5878, chance = 100000}, -- minotaur leather
	{id = 2152, chance = 80000, maxCount = 3}, -- platinum coin
	{id = 2513, chance = 40000}, -- battle shield
	{id = 2465, chance = 60000}, -- brass armor
	{id = 2648, chance = 40000}, -- chain legs
	{id = 2387, chance = 60000}, -- double axe
	{id = 7401, chance = 40000}, -- minotaur trophy
	{id = 7363, chance = 40000, maxCount = 7}, -- piercing bolt
	{id = 2666, chance = 20000, maxCount = 3}, -- meat
	{id = 2547, chance = 20000, maxCount = 7}, -- power bolt
	{id = 7588, chance = 20000}, -- strong health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -170, target = false},
	{name = "combat", interval = 1000, chance = 12, minDamage = 0, maxDamage = -120, range = 7, shootEffect = CONST_ANI_BOLT, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1000, chance = 10, minDamage = 0, maxDamage = -80, radius = 3, effect = CONST_ME_BLACKSPARK, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 22,
	armor = 16,
	{name = "combat", interval = 1000, chance = 15, minDamage = 50, maxDamage = 100, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 275, duration = 5000},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Minotaur Archer", chance = 15, interval = 1000, max = 2},
	{name = "Minotaur Guard", chance = 12, interval = 1000, max = 2},
}

mType:register(monster)