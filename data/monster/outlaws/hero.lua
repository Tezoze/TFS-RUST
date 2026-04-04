local mType = Game.createMonsterType("Hero")
local monster = {}

monster.description = "a hero"
monster.experience = 1200
monster.outfit = {
	lookType = 73,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 1400
monster.maxHealth = 1400
monster.race = "blood"
monster.speed = 280
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnFire = false,
	canWalkOnEnergy = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Let's have a fight!", yell = false},
	{text = "Welcome to my battleground!", yell = false},
	{text = "Have you seen princess Lumelia?", yell = false},
	{text = "I will sing a tune at your grave.", yell = false},
}

monster.loot = {
	{id = 1949, chance = 45000}, -- scroll
	{id = 2071, chance = 1640}, -- lyre
	{id = 2114, chance = 80}, -- piggy bank
	{id = 2120, chance = 2190}, -- rope
	{id = 2121, chance = 4910}, -- wedding ring
	{id = 2148, chance = 59500, maxCount = 100}, -- gold coin
	{id = 2164, chance = 470}, -- might ring
	{id = 2377, chance = 1500}, -- two handed sword
	{id = 2391, chance = 870}, -- war hammer
	{id = 2392, chance = 550}, -- fire sword
	{id = 2456, chance = 13300}, -- bow
	{id = 2487, chance = 490}, -- crown armor
	{id = 2488, chance = 660}, -- crown legs
	{id = 2491, chance = 450}, -- crown helmet
	{id = 2519, chance = 280}, -- crown shield
	{id = 2544, chance = 26000, maxCount = 13}, -- arrow
	{id = 2652, chance = 8000}, -- green tunic
	{id = 2661, chance = 1110}, -- scarf
	{id = 2666, chance = 8200, maxCount = 3}, -- meat
	{id = 2681, chance = 19850}, -- grapes
	{id = 2744, chance = 20450}, -- red rose
	{id = 5911, chance = 5006}, -- red piece of cloth
	{id = 7364, chance = 11400, maxCount = 4}, -- sniper arrow
	{id = 7591, chance = 720}, -- great health potion
	{id = 12406, chance = 930}, -- small notebook
	{id = 12466, chance = 5000}, -- scroll of heroic deeds
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -240, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -120, range = 7, shootEffect = CONST_ANI_ARROW, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 40,
	armor = 35,
	{name = "combat", interval = 2000, chance = 20, minDamage = 200, maxDamage = 250, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 40},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_EARTHDAMAGE, percent = 50},
	{type = COMBAT_PHYSICALDAMAGE, percent = 30},
	{type = COMBAT_DEATHDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)