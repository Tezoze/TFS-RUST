local mType = Game.createMonsterType("Warlord Ruzad")
local monster = {}

monster.description = "Warlord Ruzad"
monster.experience = 1700
monster.outfit = {
	lookType = 2,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6008
monster.health = 1500
monster.maxHealth = 1500
monster.race = "blood"
monster.speed = 270
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
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 12435, chance = 25000}, -- orc leather
	{id = 2148, chance = 18500, maxCount = 45}, -- gold coin
	{id = 2399, chance = 14500, maxCount = 18}, -- throwing star
	{id = 2667, chance = 11300, maxCount = 2}, -- fish
	{id = 2428, chance = 5700}, -- orcish axe
	{id = 3965, chance = 5700}, -- hunting spear
	{id = 2463, chance = 5610}, -- plate armor
	{id = 2647, chance = 4680}, -- plate legs
	{id = 2419, chance = 4050}, -- scimitar
	{id = 2200, chance = 2690}, -- protection amulet
	{id = 2377, chance = 2200}, -- two handed sword
	{id = 2490, chance = 1900}, -- dark helmet
	{id = 7891, chance = 750}, -- magma boots
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -300, target = false},
}

monster.defenses = {
	defense = 35,
	armor = 32,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 80},
	{type = COMBAT_ENERGYDAMAGE, percent = 2},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Orc Berserker", chance = 30, interval = 2000, max = 2},
}

mType:register(monster)