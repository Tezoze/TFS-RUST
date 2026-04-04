local mType = Game.createMonsterType("Massive Fire Elemental")
local monster = {}

monster.description = "a massive fire elemental"
monster.experience = 1400
monster.outfit = {
	lookType = 242,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6324
monster.health = 1200
monster.maxHealth = 1200
monster.race = "fire"
monster.speed = 238
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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.loot = {
	{id = 2147, chance = 6100, maxCount = 2}, -- small ruby
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 25000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 25000, maxCount = 12}, -- gold coin
	{id = 2187, chance = 2240}, -- wand of inferno
	{id = 2392, chance = 530}, -- fire sword
	{id = 7890, chance = 1300}, -- magma amulet
	{id = 7891, chance = 560}, -- magma boots
	{id = 7894, chance = 210}, -- magma legs
	{id = 9809, chance = 1330},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -300, target = false},
	{name = "firefield", interval = 2000, chance = 10, range = 7, radius = 3, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "firefield", interval = 2000, chance = 10, range = 7, radius = 3, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "combat", interval = 2000, chance = 10, minDamage = -200, maxDamage = -700, target = false, length = 7, spread = 0, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -250, radius = 3, effect = CONST_ME_EXPLOSION, target = false, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 15, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 57,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = -15},
	{type = COMBAT_PHYSICALDAMAGE, percent = 40},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)