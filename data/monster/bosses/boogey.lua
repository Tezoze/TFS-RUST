local mType = Game.createMonsterType("Boogey")
local monster = {}

monster.description = "Boogey"
monster.experience = 475
monster.outfit = {
	lookType = 300,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8955
monster.health = 930
monster.maxHealth = 930
monster.race = "undead"
monster.speed = 400
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 5000,
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
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Since you didn't eat your spinach Bogey comes to get you!", yell = true},
	{text = "Too bad you did not eat your lunch, now I have to punish you!", yell = true},
	{text = "Even if you beat me, I'll hide in your closet until you one day drop your guard!", yell = true},
	{text = "You better had believe in me!", yell = true},
	{text = "I'll take you into the darkness ... forever!", yell = true},
}

monster.loot = {
	{id = 10296, chance = 1000}, -- heavy metal t-shirt
	{id = 10302, chance = 1000}, -- club of the fury
	{id = 10301, chance = 1000}, -- scythe of the reaper
	{id = 10295, chance = 1000}, -- musician's bow
}

monster.attacks = {
	{name = "melee", interval = 1200, minDamage = 0, maxDamage = -120, target = false},
	{name = "combat", interval = 1500, chance = 30, minDamage = 0, maxDamage = -30, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 1500, chance = 30, minDamage = -12, maxDamage = -20, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_REDSPARK, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 1500, chance = 40, minDamage = -20, maxDamage = -30, effect = CONST_ME_MORTAREA, target = false, spread = 3, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 2000, chance = 25, minDamage = 80, maxDamage = 120, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 40},
	{type = COMBAT_ICEDAMAGE, percent = 25},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Demon Skeleton", chance = 40, interval = 4000, max = 2},
}

mType:register(monster)