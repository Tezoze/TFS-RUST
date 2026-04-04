local mType = Game.createMonsterType("Warlock")
local monster = {}

monster.description = "a warlock"
monster.experience = 4000
monster.outfit = {
	lookType = 130,
	lookHead = 19,
	lookBody = 71,
	lookLegs = 128,
	lookFeet = 128,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 3500
monster.maxHealth = 3500
monster.race = "blood"
monster.speed = 230
monster.manaCost = 0
monster.maxSummons = 1

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
	targetDistance = 4,
	staticAttackChance = 90,
	runHealth = 900,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Learn the secret of our magic! YOUR death!", yell = false},
	{text = "Even a rat is a better mage than you!", yell = false},
	{text = "We don't like intruders!", yell = false},
}

monster.loot = {
	{id = 1986, chance = 300}, -- red tome
	{id = 2047, chance = 1500}, -- candlestick
	{id = 2114, chance = 60}, -- piggy bank
	{id = 2123, chance = 430}, -- ring of the sky
	{id = 2124, chance = 700}, -- crystal ring
	{id = 2146, chance = 1190}, -- small sapphire
	{id = 2148, chance = 29340, maxCount = 80}, -- gold coin
	{id = 2151, chance = 1150}, -- talon
	{id = 2167, chance = 2200}, -- energy ring
	{id = 2178, chance = 2000}, -- mind stone
	{id = 2197, chance = 330}, -- stone skin amulet
	{id = 2411, chance = 7600}, -- poison dagger
	{id = 2436, chance = 6370}, -- skull staff
	{id = 2466, chance = 240}, -- golden armor
	{id = 2600, chance = 1000}, -- inkwell
	{id = 2656, chance = 1410}, -- blue robe
	{id = 2679, chance = 19000, maxCount = 4}, -- cherry
	{id = 2689, chance = 9000}, -- bread
	{id = 2792, chance = 3000}, -- dark mushroom
	{id = 7368, chance = 3500, maxCount = 4}, -- assassin star
	{id = 7368, chance = 3470, maxCount = 4}, -- assassin star
	{id = 7590, chance = 4760}, -- great mana potion
	{id = 7591, chance = 5190}, -- great health potion
	{id = 7898, chance = 1000}, -- lightning robe
	{id = 12410, chance = 510}, -- luminous orb
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -130, target = false},
	{name = "combat", interval = 2000, chance = 25, minDamage = -90, maxDamage = -180, range = 7, shootEffect = CONST_ANI_ENERGY, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 5, range = 5, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = 0, maxDamage = -120, range = 7, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 20, minDamage = -50, maxDamage = -180, range = 7, radius = 3, shootEffect = CONST_ANI_BURSTARROW, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "firefield", interval = 2000, chance = 10, range = 7, radius = 2, shootEffect = CONST_ANI_FIRE, target = true},
	{name = "combat", interval = 2000, chance = 10, minDamage = -150, maxDamage = -230, effect = CONST_ME_BIGCLOUDS, target = false, length = 8, spread = 0, type = COMBAT_ENERGYDAMAGE},
	{name = "speed", interval = 2000, chance = 15, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -600, duration = 20000},
}

monster.defenses = {
	defense = 20,
	armor = 33,
	{name = "combat", interval = 2000, chance = 20, minDamage = 100, maxDamage = 225, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 95},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_PHYSICALDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "energy", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "stone golem", chance = 10, interval = 2000, max = 1},
}

mType:register(monster)