local mType = Game.createMonsterType("Fire Devil")
local monster = {}

monster.description = "a fire devil"
monster.experience = 145
monster.outfit = {
	lookType = 40,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5985
monster.health = 200
monster.maxHealth = 200
monster.race = "blood"
monster.speed = 180
monster.manaCost = 530
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Hot, eh?", yell = false},
	{text = "Hell, oh, hell!", yell = false},
}

monster.loot = {
	{id = 2050, chance = 10000}, -- torch
	{id = 2050, chance = 1420, maxCount = 2}, -- torch
	{id = 2150, chance = 300}, -- small amethyst
	{id = 2185, chance = 460}, -- necrotic rod
	{id = 2260, chance = 10950}, -- blank rune
	{id = 2387, chance = 1500}, -- double axe
	{id = 2419, chance = 3000}, -- scimitar
	{id = 2515, chance = 210}, -- guardian shield
	{id = 2568, chance = 1100}, -- cleaver
	{id = 12469, chance = 19770}, -- small pitchfork
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -35, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -60, maxDamage = -90, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -20, maxDamage = -50, range = 7, radius = 2, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 10,
	armor = 13,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -20},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)