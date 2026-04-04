local mType = Game.createMonsterType("Evil Mastermind")
local monster = {}

monster.description = "Evil Mastermind"
monster.experience = 675
monster.outfit = {
	lookType = 256,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7256
monster.health = 1295
monster.maxHealth = 1295
monster.race = "undead"
monster.speed = 350
monster.manaCost = 0
monster.maxSummons = 1

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
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "You won't stop my masterplan to flood the world market with fake Bonelord language dictionaries!", yell = false},
	{text = "My calculations tell me you'll die!", yell = false},
	{text = "You can't stop me!", yell = false},
	{text = "Beware! My evil monolog is coming!", yell = false},
}

monster.loot = {
	{id = 10308, chance = 10000}, -- fan club membership card
	{id = 2148, chance = 100000, maxCount = 95}, -- gold coin
	{id = 2152, chance = 93000, maxCount = 3}, -- platinum coin
}

monster.attacks = {
	{name = "melee", interval = 1200, minDamage = 0, maxDamage = -77, target = false},
	{name = "combat", interval = 2000, chance = 30, minDamage = -50, maxDamage = -78, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 30, minDamage = -66, maxDamage = -72, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 30, minDamage = -36, maxDamage = -57, range = 7, shootEffect = CONST_ANI_ENERGY, effect = CONST_ME_ENERGYAREA, target = true, type = COMBAT_ENERGYDAMAGE},
	{name = "combat", interval = 2000, chance = 30, minDamage = -70, maxDamage = -73, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 30, minDamage = -59, maxDamage = -75, range = 7, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_MANADRAIN},
	{name = "speed", interval = 2000, chance = 15, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -600, duration = 20000},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 2000, chance = 30, minDamage = 50, maxDamage = 110, effect = CONST_ME_ENERGY, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_ENERGYDAMAGE, percent = 90},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "vampire", chance = 40, interval = 2000, max = 3},
}

mType:register(monster)