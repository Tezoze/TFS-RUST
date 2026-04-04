local mType = Game.createMonsterType("Blistering Fire Elemental")
local monster = {}

monster.description = "a blistering fire elemental"
monster.experience = 1300
monster.outfit = {
	lookType = 242,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8964
monster.health = 1500
monster.maxHealth = 1500
monster.race = "fire"
monster.speed = 230
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

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "FCHHHRRR", yell = false},
}

monster.loot = {
	{id = 2147, chance = 3200, maxCount = 3}, -- small ruby
	{id = 2148, chance = 12500, maxCount = 65}, -- gold coin
	{id = 2148, chance = 12500, maxCount = 60}, -- gold coin
	{id = 8299, chance = 2500}, -- glimmering soil
	{id = 8921, chance = 1250}, -- wand of draconia
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -350, interval = 2000, target = false},
	{name = "combat", type = COMBAT_FIREDAMAGE, minDamage = -65, maxDamage = -510, interval = 1000, chance = 11, length = 7, spread = 3, target = false},
	{name = "condition", type = CONDITION_FIRE, interval = 1000, chance = 12, tick = 10000, minDamage = -50, maxDamage = -200, radius = 6, effect = CONST_ME_FIREAREA, target = false},
	{name = "firefield", interval = 1000, chance = 15, range = 7, radius = 3, target = true, shootEffect = CONST_ANI_FIRE},
}

monster.defenses = {
	defense = 20,
	armor = 20,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)