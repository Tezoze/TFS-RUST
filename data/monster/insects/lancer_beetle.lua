local mType = Game.createMonsterType("Lancer Beetle")
local monster = {}

monster.description = "a lancer beetle"
monster.experience = 275
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
monster.speed = 266
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 30,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Crump!", yell = true},
}

monster.loot = {
	{id = 2148, chance = 60000, maxCount = 61}, -- gold coin
	{id = 2148, chance = 60000, maxCount = 79}, -- gold coin
	{id = 2150, chance = 247}, -- small amethyst
	{id = 10557, chance = 8333}, -- poisonous slime
	{id = 10609, chance = 4166}, -- lump of dirt
	{id = 11372, chance = 16666}, -- lancer beetle shell
	{id = 11374, chance = 1123}, -- beetle necklace
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -115, interval = 2000, target = false},
	{name = "poisonfield", interval = 2000, chance = 10, radius = 4, target = false, effect = CONST_ME_POISON},
	{name = "combat", type = COMBAT_LIFEDRAIN, minDamage = 0, maxDamage = -90, interval = 2000, chance = 15, length = 7, spread = 0, target = false, effect = CONST_ME_GREENSPARK},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -40, maxDamage = -80, range = 7, shootEffect = CONST_ANI_POISON, target = true},
	{name = "lancer beetle curse", interval = 2000, chance = 5, range = 5, target = true},
}

monster.defenses = {
	defense = 20,
	armor = 35,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_GROUNDSHAKER},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 50},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)