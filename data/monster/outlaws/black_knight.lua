local mType = Game.createMonsterType("Black Knight")
local monster = {}

monster.description = "a black knight"
monster.experience = 1600
monster.outfit = {
	lookType = 131,
	lookHead = 95,
	lookBody = 95,
	lookLegs = 95,
	lookFeet = 95,
	lookAddons = 3,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 1800
monster.maxHealth = 1800
monster.race = "blood"
monster.speed = 250
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
	{text = "MINE!", yell = true},
	{text = "NO PRISONERS!", yell = true},
	{text = "NO MERCY!", yell = true},
	{text = "By Bolg's blood!", yell = false},
	{text = "You're no match for me!", yell = false},
}

monster.loot = {
	{id = 2114, chance = 210}, -- piggy bank
	{id = 2120, chance = 16020}, -- rope
	{id = 2133, chance = 740}, -- ruby necklace
	{id = 2148, chance = 23000, maxCount = 80}, -- gold coin
	{id = 2148, chance = 23000, maxCount = 56}, -- gold coin
	{id = 2195, chance = 320}, -- boots of haste
	{id = 2377, chance = 8470}, -- two handed sword
	{id = 2381, chance = 11850}, -- halberd
	{id = 2389, chance = 30800, maxCount = 3}, -- spear
	{id = 2414, chance = 110}, -- dragon lance
	{id = 2417, chance = 6980}, -- battle hammer
	{id = 2430, chance = 3280}, -- knight axe
	{id = 2457, chance = 11220}, -- steel helmet
	{id = 2463, chance = 10370}, -- plate armor
	{id = 2475, chance = 5610}, -- warrior helmet
	{id = 2476, chance = 320}, -- knight armor
	{id = 2477, chance = 1050}, -- knight legs
	{id = 2478, chance = 12200}, -- brass legs
	{id = 2489, chance = 1900}, -- dark armor
	{id = 2490, chance = 2330}, -- dark helmet
	{id = 2691, chance = 21600, maxCount = 2}, -- brown bread
	{id = 7895, chance = 950}, -- lightning legs
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -300, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -200, range = 7, shootEffect = CONST_ANI_SPEAR, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 40,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 95},
	{type = COMBAT_ENERGYDAMAGE, percent = 80},
	{type = COMBAT_PHYSICALDAMAGE, percent = 20},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
}


mType:register(monster)