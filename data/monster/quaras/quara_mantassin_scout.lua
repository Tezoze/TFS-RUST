local mType = Game.createMonsterType("Quara Mantassin Scout")
local monster = {}

monster.description = "a quara mantassin scout"
monster.experience = 100
monster.outfit = {
	lookType = 72,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6064
monster.health = 220
monster.maxHealth = 220
monster.race = "blood"
monster.speed = 140
monster.manaCost = 480
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
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Zuerk Pachak!", yell = false},
	{text = "Shrrrr", yell = false},
}

monster.loot = {
	{id = 2146, chance = 920}, -- small sapphire
	{id = 2148, chance = 94000, maxCount = 30}, -- gold coin
	{id = 2165, chance = 520}, -- stealth ring
	{id = 2229, chance = 920}, -- skull
	{id = 2377, chance = 580}, -- two handed sword
	{id = 2464, chance = 4761}, -- chain armor
	{id = 5895, chance = 5940, maxCount = 2}, -- fish fin
	{id = 12445, chance = 7780}, -- mantassin tail
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -110, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 7,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)