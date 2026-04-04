local mType = Game.createMonsterType("Kollos")
local monster = {}

monster.description = "a kollos"
monster.experience = 2400
monster.outfit = {
	lookType = 458,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 15354
monster.health = 3800
monster.maxHealth = 3800
monster.race = "venom"
monster.speed = 200
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
	{text = "Zopp!", yell = false},
	{text = "Flzlzlzlzlzlz!", yell = false},
}

monster.loot = {
	{id = 2147, chance = 8180, maxCount = 2}, -- small ruby
	{id = 2148, chance = 50000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 50000, maxCount = 97}, -- gold coin
	{id = 2152, chance = 66000, maxCount = 3}, -- platinum coin
	{id = 2214, chance = 3270}, -- ring of healing
	{id = 2645, chance = 130}, -- steel boots
	{id = 7590, chance = 8950, maxCount = 4}, -- great mana potion
	{id = 7632, chance = 2600}, -- giant shimmering pearl
	{id = 8473, chance = 4000, maxCount = 3}, -- ultimate health potion
	{id = 9971, chance = 5160}, -- gold ingot
	{id = 15480, chance = 15390}, -- kollos shell
	{id = 15486, chance = 15720}, -- compound eye
	{id = 15489, chance = 360}, -- calopteryx cape
	{id = 15491, chance = 310}, -- carapace shield
	{id = 15492, chance = 700}, -- hive scythe
	{id = 15646, chance = 460}, -- buggy backpack
	{id = 15648, chance = 10210, maxCount = 5}, -- tarsal arrow
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -315, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -500, radius = 3, shootEffect = CONST_ANI_EXPLOSION, effect = CONST_ME_EXPLOSIONHIT, target = true, range = 7, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 35,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = 5},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)
