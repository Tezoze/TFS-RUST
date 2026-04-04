local mType = Game.createMonsterType("Mephiles")
local monster = {}

monster.description = "Mephiles"
monster.experience = 415
monster.outfit = {
	lookType = 237,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6364
monster.health = 415
monster.maxHealth = 415
monster.race = "blood"
monster.speed = 300
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 3,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "I have a contract here which you should sign!", yell = false},
	{text = "I sence so much potential in you. It's almost a shame I have to kill you.", yell = false},
	{text = "Yes, slay me for the loot I might have. Give in to your greed.", yell = false},
	{text = "Wealth, Power, it is all at your fingertips. All you have to do is a bit blackmailing and bullying.", yell = false},
	{text = "Come on. being a bit evil won't hurt you.", yell = false},
}

monster.loot = {
	{id = 2148, chance = 2000, maxCount = 95}, -- gold coin
	{id = 2152, chance = 30000, maxCount = 9}, -- platinum coin
	{id = 10293, chance = 1000}, -- stale bread of ancientness
	{id = 10304, chance = 1000}, -- poet's fencing quill
	{id = 10317, chance = 1000}, -- rain coat
	{id = 10294, chance = 1000}, -- shield of the white knight
}

monster.attacks = {
	{name = "melee", interval = 1200, minDamage = 0, maxDamage = -35, target = false},
	{name = "combat", interval = 1500, chance = 70, minDamage = -15, maxDamage = -45, range = 7, radius = 2, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 30,
	{name = "speed", interval = 1000, chance = 40, effect = CONST_ME_REDSHIMMER, speed = 400, duration = 40000},
	{name = "invisible", interval = 4000, chance = 50, effect = CONST_ME_REDSHIMMER},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 50},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)