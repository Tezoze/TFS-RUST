local mType = Game.createMonsterType("The Blightfather")
local monster = {}

monster.description = "the Blightfather"
monster.experience = 400
monster.outfit = {
	lookType = 348,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 11375
monster.health = 400
monster.maxHealth = 400
monster.race = "venom"
monster.speed = 290
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 12
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
	runHealth = 80,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 2000, maxCount = 61}, -- gold coin
	{id = 2148, chance = 2000, maxCount = 60}, -- gold coin
	{id = 10609, chance = 17500}, -- lump of dirt
	{id = 10557, chance = 12500}, -- poisonous slime
	{id = 11372, chance = 7000}, -- lancer beetle shell
	{id = 11374, chance = 400}, -- beetle necklace
	{id = 2150, chance = 800}, -- small amethyst
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 40, attack = 80, target = false},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "invisible", interval = 1000, chance = 10, effect = CONST_ME_REDSHIMMER},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)