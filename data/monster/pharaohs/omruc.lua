local mType = Game.createMonsterType("Omruc")
local monster = {}

monster.description = "Omruc"
monster.experience = 2950
monster.outfit = {
	lookType = 90,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6025
monster.health = 4300
monster.maxHealth = 4300
monster.race = "undead"
monster.speed = 370
monster.manaCost = 0
monster.maxSummons = 4

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
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Now chhhou shhhee me ... Now chhhou don't.", yell = false},
	{text = "Chhhhou are marked ashhh my prey.", yell = false},
	{text = "Catchhhh me if chhhou can.", yell = false},
	{text = "Die!", yell = false},
	{text = "Psssst, I am over chhhere.", yell = false},
}

monster.loot = {
	{id = 2145, chance = 7000, maxCount = 3}, -- small diamond
	{id = 2148, chance = 50000, maxCount = 90}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 70}, -- gold coin
	{id = 2154, chance = 5000}, -- yellow gem
	{id = 2165, chance = 5000}, -- stealth ring
	{id = 2195, chance = 1500}, -- boots of haste
	{id = 2352, chance = 100000}, -- crystal arrow
	{id = 2544, chance = 10000, maxCount = 21}, -- arrow
	{id = 2545, chance = 10000, maxCount = 20}, -- poison arrow
	{id = 2546, chance = 10000, maxCount = 15}, -- burst arrow
	{id = 2547, chance = 10000, maxCount = 3}, -- power bolt
	{id = 7365, chance = 10000, maxCount = 2}, -- onyx arrow
	{id = 7591, chance = 7000}, -- great health potion
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -120, target = false, condition = {type = CONDITION_POISON, startDamage = 65, interval = 2000}},
	{name = "combat", interval = 5000, chance = 20, minDamage = -100, maxDamage = -250, range = 1, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 20, minDamage = -200, maxDamage = -500, range = 7, shootEffect = CONST_ANI_POISONARROW, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 1000, chance = 20, minDamage = -120, maxDamage = -450, range = 3, shootEffect = CONST_ANI_BURSTARROW, effect = CONST_ME_EXPLOSIONAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "melee", interval = 3000, chance = 20, minDamage = -150, maxDamage = -500, shootEffect = CONST_ANI_ARROW, target = true},
	{name = "speed", interval = 1000, chance = 25, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -900, duration = 50000},
}

monster.defenses = {
	defense = 35,
	armor = 20,
	{name = "combat", interval = 1000, chance = 17, minDamage = 100, maxDamage = 200, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.immunities = {
	{type = "death", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Stalker", chance = 100, interval = 2000, max = 4},
}

mType:register(monster)