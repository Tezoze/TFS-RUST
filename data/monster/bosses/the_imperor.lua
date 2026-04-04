local mType = Game.createMonsterType("The Imperor")
local monster = {}

monster.description = "The Imperor"
monster.experience = 8000
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
monster.health = 15000
monster.maxHealth = 15000
monster.race = "blood"
monster.speed = 310
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 5
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
	runHealth = 1500,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "Poke! Poke! <chuckle>", yell = false},
	{text = "Let me tickle you with my trident!", yell = false},
}

monster.loot = {
	{id = 6500, chance = 100000}, -- demonic essence
	{id = 2148, chance = 100000, maxCount = 150}, -- gold coin
	{id = 6534, chance = 100000}, -- the imperor's trident
	{id = 2548, chance = 53850}, -- pitchfork
	{id = 2432, chance = 11000}, -- fire axe
	{id = 2152, chance = 46150, maxCount = 3}, -- platinum coin
	{id = 5944, chance = 100000}, -- soul orb
	{id = 2488, chance = 30770}, -- crown legs
	{id = 2470, chance = 7690}, -- golden legs
	{id = 2136, chance = 15380}, -- demonbone amulet
	{id = 2542, chance = 7690}, -- tempest shield
	{id = 2515, chance = 15400}, -- guardian shield
	{id = 7899, chance = 15380}, -- magma coat
	{id = 2150, chance = 30770, maxCount = 4}, -- small amethyst
	{id = 2147, chance = 7690, maxCount = 4}, -- small ruby
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 90, attack = 100, target = false, condition = {type = CONDITION_POISON, startDamage = 280, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -100, maxDamage = -350, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2500, chance = 12, minDamage = -100, maxDamage = -460, range = 7, radius = 2, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREATTACK, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, range = 7, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 35, minDamage = 275, maxDamage = 840, effect = CONST_ME_REDSHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 12, effect = CONST_ME_REDSHIMMER, speed = 1496, duration = 5000},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_TELEPORT},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 50},
	{type = COMBAT_PHYSICALDAMAGE, percent = 30},
	{type = COMBAT_EARTHDAMAGE, percent = 50},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)