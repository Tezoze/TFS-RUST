local mType = Game.createMonsterType("Spidris Elite")
local monster = {}

monster.description = "a spidris elite"
monster.experience = 4000
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
monster.health = 5000
monster.maxHealth = 5000
monster.race = "venom"
monster.speed = 260
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
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

monster.loot = {
	{id = 2147, chance = 23280, maxCount = 5}, -- small ruby
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2152, chance = 45000, maxCount = 6}, -- platinum coin
	{id = 2153, chance = 1120}, -- violet gem
	{id = 6300, chance = 4480}, -- death ring
	{id = 7413, chance = 1440}, -- titan axe
	{id = 7590, chance = 20400, maxCount = 2}, -- great mana potion
	{id = 7632, chance = 3040}, -- giant shimmering pearl
	{id = 8473, chance = 9250, maxCount = 2}, -- ultimate health potion
	{id = 15485, chance = 27440}, -- spidris mandible
	{id = 15486, chance = 13210}, -- compound eye
	{id = 15489, chance = 1280}, -- calopteryx cape
	{id = 15491, chance = 1170}, -- carapace shield
	{id = 15492, chance = 1390}, -- hive scythe
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -349, target = false},
}

monster.defenses = {
	defense = 30,
	armor = 30,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
