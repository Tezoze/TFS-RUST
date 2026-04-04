local mType = Game.createMonsterType("War Golem")
local monster = {}

monster.description = "a war golem"
monster.experience = 2750
monster.outfit = {
	lookType = 326,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 10005
monster.health = 4300
monster.maxHealth = 4300
monster.race = "energy"
monster.speed = 320
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
	interval = 2500,
	chance = 10,
	{text = "Azerus barada nikto!", yell = false},
	{text = "Klonk klonk klonk", yell = false},
	{text = "Engaging Enemy!", yell = false},
	{text = "Threat level processed.", yell = false},
	{text = "Charging weapon systems!", yell = false},
	{text = "Auto repair in progress.", yell = false},
	{text = "The battle is joined!", yell = false},
	{text = "Termination initialized!", yell = false},
	{text = "Rrrtttarrrttarrrtta", yell = false},
	{text = "Eliminated", yell = false},
}

monster.loot = {
	{id = 2148, chance = 37500, maxCount = 90}, -- gold coin
	{id = 2148, chance = 37500, maxCount = 80}, -- gold coin
	{id = 8309, chance = 5260, maxCount = 5}, -- nail
	{id = 2377, chance = 5500}, -- two handed sword
	{id = 2510, chance = 9000}, -- plate shield
	{id = 2394, chance = 7000}, -- morning star
	{id = 2513, chance = 6500}, -- battle shield
	{id = 8473, chance = 10080}, -- ultimate health potion
	{id = 7590, chance = 8860}, -- great mana potion
	{id = 5880, chance = 1920}, -- iron ore
	{id = 7439, chance = 900}, -- berserk potion
	{id = 2438, chance = 6400}, -- epee
	{id = 9809, chance = 260},
	{id = 2207, chance = 810}, -- melee ring
	{id = 2213, chance = 1210}, -- dwarven ring
	{id = 9980, chance = 130}, -- crystal of power
	{id = 9978, chance = 1080}, -- crystal pedestal
	{id = 7893, chance = 90}, -- lightning boots
	{id = 2645, chance = 620}, -- steel boots
	{id = 7403, chance = 50}, -- berserker
	{id = 7422, chance = 120}, -- jade hammer
	{id = 7428, chance = 770}, -- bonebreaker
	{id = 2177, chance = 1000}, -- life crystal
	{id = 13292, chance = 100}, -- tin key
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -550, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -165, maxDamage = -220, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "outfit", interval = 2000, chance = 1, range = 7, target = false},
	{name = "combat", interval = 2000, chance = 15, range = 1, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 35,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 300, duration = 5000},
	{name = "combat", interval = 2000, chance = 15, minDamage = 200, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_FIREDAMAGE, percent = 15},
	{type = COMBAT_ENERGYDAMAGE, percent = 5},
	{type = COMBAT_PHYSICALDAMAGE, percent = 25},
	{type = COMBAT_ICEDAMAGE, percent = 30},
	{type = COMBAT_DEATHDAMAGE, percent = 25},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)