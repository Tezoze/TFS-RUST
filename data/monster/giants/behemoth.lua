local mType = Game.createMonsterType("Behemoth")
local monster = {}

monster.description = "a behemoth"
monster.experience = 2500
monster.outfit = {
	lookType = 55,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5999
monster.health = 4000
monster.maxHealth = 4000
monster.race = "blood"
monster.speed = 340
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
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 70,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "You're so little!", yell = false},
	{text = "Human flesh -  delicious!", yell = false},
	{text = "Crush the intruders!", yell = false},
}

monster.loot = {
	{id = 2023, chance = 100}, -- amphora
	{id = 2125, chance = 2530}, -- crystal necklace
	{id = 2148, chance = 59530, maxCount = 100}, -- gold coin
	{id = 2148, chance = 40000, maxCount = 99}, -- gold coin
	{id = 2150, chance = 6380, maxCount = 5}, -- small amethyst
	{id = 2152, chance = 59800, maxCount = 5}, -- platinum coin
	{id = 2174, chance = 750}, -- strange symbol
	{id = 2231, chance = 670}, -- big bone
	{id = 2377, chance = 5980}, -- two handed sword
	{id = 2387, chance = 10510}, -- double axe
	{id = 2393, chance = 1006}, -- giant sword
	{id = 2416, chance = 100}, -- crowbar
	{id = 2454, chance = 50}, -- war axe
	{id = 2463, chance = 3930}, -- plate armor
	{id = 2489, chance = 4370}, -- dark armor
	{id = 2553, chance = 650}, -- pick
	{id = 2645, chance = 380}, -- steel boots
	{id = 2666, chance = 30000, maxCount = 6}, -- meat
	{id = 5893, chance = 3090}, -- perfect behemoth fang
	{id = 5930, chance = 430}, -- behemoth claw
	{id = 7368, chance = 9750, maxCount = 5}, -- assassin star
	{id = 7396, chance = 170}, -- behemoth trophy
	{id = 7413, chance = 90}, -- titan axe
	{id = 7591, chance = 5120}, -- great health potion
	{id = 12403, chance = 14000}, -- battle stone
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -450, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -200, range = 7, shootEffect = CONST_ANI_LARGEROCK, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 45,
	armor = 50,
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 300, duration = 5000},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 80},
	{type = COMBAT_HOLYDAMAGE, percent = 30},
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
	{type = COMBAT_DEATHDAMAGE, percent = -5},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)