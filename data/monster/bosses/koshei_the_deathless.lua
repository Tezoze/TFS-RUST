local mType = Game.createMonsterType("Koshei The Deathless")
local monster = {}

monster.description = "Koshei the Deathless"
monster.experience = 0
monster.outfit = {
	lookType = 99,
	lookHead = 95,
	lookBody = 116,
	lookLegs = 119,
	lookFeet = 115,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8272
monster.health = 3000
monster.maxHealth = 3000
monster.race = "undead"
monster.speed = 390
monster.manaCost = 0
monster.maxSummons = 1

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
	interval = 5000,
	chance = 10,
	{text = "Your pain will be beyond imagination!", yell = false},
	{text = "You can't defeat me! I will resurrect and take your soul!", yell = false},
	{text = "Death is my ally!", yell = false},
	{text = "Welcome to my domain visitor!", yell = false},
	{text = "You will be my toy on the other side!", yell = false},
	{text = "You will endure agony beyond thy death!", yell = false},
	{text = "What a disgusting smell of life!", yell = false},
	{text = "Ahhh, your life energy tastes so delicious!", yell = false},
}

monster.loot = {
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -170, interval = 2000, target = false},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -60, maxDamage = -250, interval = 3000, chance = 9, range = 1, target = true, effect = CONST_ME_BLUESHIMMER},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = -70, maxDamage = -135, interval = 1000, chance = 11, radius = 3, target = false, effect = CONST_ME_REDSHIMMER},
	{name = "combat", type = COMBAT_DEATHDAMAGE, minDamage = -50, maxDamage = -140, interval = 2000, chance = 9, length = 8, spread = 0, target = false, effect = CONST_ME_MORTAREA},
	{name = "condition", type = CONDITION_CURSED, interval = 3000, chance = 15, tick = 4000, minDamage = -54, maxDamage = -54, range = 1, target = true},
	{name = "speed", interval = 2000, chance = 15, range = 7, target = true, effect = CONST_ME_REDSHIMMER, speed = -900, duration = 30000},
}

monster.defenses = {
	defense = 20,
	armor = 20,
	{name = "combat", interval = 1000, chance = 30, minDamage = 150, maxDamage = 300, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 80},
	{type = COMBAT_HOLYDAMAGE, percent = -15},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "bonebeast", chance = 16, interval = 1000, max = 1},
}

mType:register(monster)