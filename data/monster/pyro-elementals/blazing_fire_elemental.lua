local mType = Game.createMonsterType("Blazing Fire Elemental")
local monster = {}

monster.description = "a blazing fire elemental"
monster.experience = 450
monster.outfit = {
	lookType = 49,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8964
monster.health = 650
monster.maxHealth = 650
monster.race = "fire"
monster.speed = 220
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.loot = {
	{id = 2148, chance = 10000, maxCount = 40}, -- gold coin
	{id = 7840, chance = 5000, maxCount = 4}, -- flaming arrow
	{id = 8299, chance = 2500}, -- glimmering soil
	{id = 10553, chance = 5475}, -- fiery heart
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 1000, chance = 13, minDamage = -65, maxDamage = -205, radius = 5, effect = CONST_ME_FIREAREA, target = false, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 1000, chance = 12, minDamage = -110, maxDamage = -150, range = 7, radius = 5, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "firefield", interval = 1000, chance = 15, range = 7, radius = 1, shootEffect = CONST_ANI_FIRE, target = true},
}

monster.defenses = {
	defense = 20,
	armor = 20,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)