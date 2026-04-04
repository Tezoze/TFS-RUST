local mType = Game.createMonsterType("Demodras")
local monster = {}

monster.description = "Demodras"
monster.experience = 6000
monster.outfit = {
	lookType = 204,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5984
monster.health = 4500
monster.maxHealth = 4500
monster.race = "blood"
monster.speed = 230
monster.manaCost = 0
monster.maxSummons = 2

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
	runHealth = 300,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I WILL SET THE WORLD ON FIRE!", yell = true},
	{text = "I WILL PROTECT MY BROOD!", yell = true},
}

monster.loot = {
	{id = 2672, chance = 20000, maxCount = 2}, -- dragon ham
	{id = 2033, chance = 1818}, -- golden mug
	{id = 1976, chance = 3333}, -- book
	{id = 2492, chance = 333}, -- dragon scale mail
	{id = 2547, chance = 2222, maxCount = 10}, -- power bolt
	{id = 2796, chance = 6666}, -- green mushroom
	{id = 2392, chance = 1428}, -- fire sword
	{id = 2146, chance = 2222, maxCount = 2}, -- small sapphire
	{id = 5948, chance = 5000}, -- red dragon leather
	{id = 5919, chance = 100000}, -- dragon claw
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = -160, maxDamage = -600, target = false},
	{name = "combat", interval = 3000, chance = 20, minDamage = -250, maxDamage = -350, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "firefield", interval = 1000, chance = 10, range = 7, radius = 6, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "combat", interval = 4000, chance = 20, minDamage = -250, maxDamage = -550, effect = CONST_ME_FIREAREA, target = false, length = 8, spread = 3, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 25,
	armor = 45,
	{name = "combat", interval = 1000, chance = 25, minDamage = 400, maxDamage = 700, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Dragon", chance = 17, interval = 1000, max = 2},
}

mType:register(monster)