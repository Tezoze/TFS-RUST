local mType = Game.createMonsterType("Spidris")
local monster = {}

monster.description = "a spidris"
monster.experience = 2600
monster.outfit = {
	lookType = 457,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15296
monster.health = 3700
monster.maxHealth = 3700
monster.race = "venom"
monster.speed = 260
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
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Eeeeeeyyyyh!", yell = false},
	{text = "Iiiiieeeeeh!", yell = false},
}

monster.loot = {
	{id = 2147, chance = 11900, maxCount = 5}, -- small ruby
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2152, chance = 45000, maxCount = 4}, -- platinum coin
	{id = 2153, chance = 770}, -- violet gem
	{id = 6300, chance = 2700}, -- death ring
	{id = 7413, chance = 920}, -- titan axe
	{id = 7590, chance = 11500, maxCount = 2}, -- great mana potion
	{id = 7632, chance = 1700}, -- giant shimmering pearl
	{id = 8473, chance = 6250, maxCount = 2}, -- ultimate health potion
	{id = 15485, chance = 14960}, -- spidris mandible
	{id = 15486, chance = 12500}, -- compound eye
	{id = 15489, chance = 370}, -- calopteryx cape
	{id = 15491, chance = 720}, -- carapace shield
	{id = 15492, chance = 690}, -- hive scythe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -298, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -150, maxDamage = -310, radius = 3, shootEffect = CONST_ANI_POISON, effect = CONST_ME_GREENBUBBLE, target = true, range = 7, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "speed", interval = 2000, chance = 15, speed = 450, effect = CONST_ME_MAGIC_RED, target = false, duration = 5000},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 5},
	{type = COMBAT_FIREDAMAGE, percent = 5},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
