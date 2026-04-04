local mType = Game.createMonsterType("Stampor")
local monster = {}

monster.description = "a stampor"
monster.experience = 780
monster.outfit = {
	lookType = 381,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 13312
monster.health = 1200
monster.maxHealth = 1200
monster.race = "blood"
monster.speed = 220
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "KRRRRRNG", yell = true},
}

monster.loot = {
	{id = 2148, chance = 30000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 30000, maxCount = 100}, -- gold coin
	{id = 2152, chance = 9920, maxCount = 2}, -- platinum coin
	{id = 2391, chance = 1010}, -- war hammer
	{id = 2476, chance = 870}, -- knight armor
	{id = 7452, chance = 160}, -- spiked squelcher
	{id = 7588, chance = 5000, maxCount = 2}, -- strong health potion
	{id = 7589, chance = 5000, maxCount = 2}, -- strong mana potion
	{id = 9057, chance = 7940, maxCount = 2}, -- small topaz
	{id = 13299, chance = 4920}, -- stampor horn
	{id = 13300, chance = 9950, maxCount = 2}, -- stampor talons
	{id = 13301, chance = 3020}, -- hollow stampor hoof
}

monster.attacks = {
	{name = "melee", interval = 2000, chance = 100, minDamage = 0, maxDamage = -130, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -150, maxDamage = -280, radius = 3, effect = CONST_ME_GROUNDSHAKER, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -75, maxDamage = -100, shootEffect = CONST_ANI_SMALLSTONE, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 0,
	armor = 60,
	{name = "combat", interval = 2000, chance = 15, minDamage = 90, maxDamage = 120, effect = CONST_ME_MAGIC_BLUE, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 30},
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
