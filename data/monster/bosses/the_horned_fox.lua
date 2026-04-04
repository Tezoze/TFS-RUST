local mType = Game.createMonsterType("The Horned Fox")
local monster = {}

monster.description = "the Horned Fox"
monster.experience = 300
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
monster.health = 265
monster.maxHealth = 265
monster.race = "blood"
monster.speed = 210
monster.manaCost = 0
monster.maxSummons = 6

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
	{text = "You will never get me!", yell = false},
	{text = "I'll be back!", yell = false},
	{text = "Catch me, if you can!", yell = false},
	{text = "Help me, Boys!", yell = false},
}

monster.loot = {
	{id = 5804, chance = 100000}, -- nose ring
	{id = 2148, chance = 96000, maxCount = 99}, -- gold coin
	{id = 5878, chance = 96000}, -- minotaur leather
	{id = 12428, chance = 92590, maxCount = 2}, -- minotaur horn
	{id = 12438, chance = 85000}, -- piece of warrior armor
	{id = 7363, chance = 48000, maxCount = 14}, -- piercing bolt
	{id = 2465, chance = 25000}, -- brass armor
	{id = 2666, chance = 18000, maxCount = 2}, -- meat
	{id = 2513, chance = 14000}, -- battle shield
	{id = 2502, chance = 5000}, -- dwarven helmet
	{id = 2580, chance = 7410}, -- fishing rod
	{id = 7588, chance = 7410}, -- strong health potion
	{id = 2387, chance = 3700}, -- double axe
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -100, interval = 2000, target = false},
	{name = "combat", type = COMBAT_PHYSICALDAMAGE, minDamage = 0, maxDamage = -20, interval = 1000, chance = 25, range = 7, target = true, shootEffect = CONST_ANI_BOLT},
	{name = "condition", type = CONDITION_POISON, interval = 1000, chance = 17, tick = 4000, minDamage = -4, maxDamage = -4, range = 7, shootEffect = CONST_ANI_POISON, target = true},
}

monster.defenses = {
	defense = 33,
	armor = 30,
	{name = "combat", interval = 1000, chance = 15, minDamage = 25, maxDamage = 75, effect = CONST_ME_REDSHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 1000, chance = 10, effect = CONST_ME_BLUESHIMMER},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Minotaur Archer", chance = 13, interval = 1000, max = 2},
	{name = "Minotaur Guard", chance = 13, interval = 1000, max = 2},
	{name = "Minotaur Mage", chance = 13, interval = 1000, max = 2},
}

mType:register(monster)