local mType = Game.createMonsterType("Yakchal")
local monster = {}

monster.description = "Yakchal"
monster.experience = 4400
monster.outfit = {
	lookType = 149,
	lookHead = 8,
	lookBody = 0,
	lookLegs = 85,
	lookFeet = 85,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 5750
monster.maxHealth = 5750
monster.race = "blood"
monster.speed = 220
monster.manaCost = 0
monster.maxSummons = 4

monster.changeTarget = {
	interval = 2000,
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
	staticAttackChance = 50,
	targetDistance = 4,
	runHealth = 100,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "YOU BETTER DIE TO MY MINIONS BECAUSE YOU'LL WISH YOU DID IF I COME FOR YOU!", yell = false},
	{text = "DESTROY THE INFIDELS", yell = false},
	{text = "You are mine!", yell = false},
	{text = "I will make you all pay!", yell = false},
	{text = "No one will stop my plans!", yell = false},
	{text = "You are responsible for this!", yell = false},
}

monster.loot = {
	{id = 7290, chance = 100000}, -- shard
	{id = 2148, chance = 97000, maxCount = 283}, -- gold coin
	{id = 5912, chance = 74000}, -- blue piece of cloth
	{id = 7440, chance = 65000}, -- mastermind potion
	{id = 9971, chance = 33000}, -- gold ingot
	{id = 7449, chance = 22000}, -- crystal sword
	{id = 2201, chance = 15000}, -- dragon necklace
	{id = 7896, chance = 12000}, -- glacier kilt
	{id = 7590, chance = 9500}, -- great mana potion
	{id = 7443, chance = 8000}, -- bullseye potion
	{id = 7897, chance = 8000}, -- glacier robe
	{id = 2436, chance = 8000}, -- skull staff
	{id = 7459, chance = 6350}, -- pair of earmuffs
	{id = 7439, chance = 4700}, -- berserk potion
	{id = 2168, chance = 4700}, -- life ring
	{id = 7410, chance = 4700}, -- queen's sceptre
	{id = 2195, chance = 1500}, -- boots of haste
	{id = 2796, chance = 1500}, -- green mushroom
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -389, target = false},
	{name = "combat", interval = 2000, chance = 18, minDamage = 0, maxDamage = -430, radius = 4, shootEffect = CONST_ANI_SMALLICE, effect = CONST_ME_ICEAREA, target = true, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 3000, chance = 34, minDamage = -200, maxDamage = -300, range = 7, radius = 3, shootEffect = CONST_ANI_SNOWBALL, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 2000, chance = 10, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -300, duration = 20000},
}

monster.defenses = {
	defense = 20,
	armor = 15,
	{name = "combat", interval = 1000, chance = 25, minDamage = 50, maxDamage = 100, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Ice Golem", chance = 13, interval = 1000, max = 4},
}

mType:register(monster)