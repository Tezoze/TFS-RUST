local mType = Game.createMonsterType("Demon")
local monster = {}

monster.description = "a demon"
monster.experience = 6000
monster.outfit = {
	lookType = 35,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.health = 8200
monster.maxHealth = 8200
monster.race = "fire"
monster.corpse = 5995
monster.speed = 128
monster.manaCost = 0
monster.maxSummons = 1

monster.changeTarget = {
	interval = 4000,
	chance = 20
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	convinceable = false,
	pushable = false,
	rewardBoss = false,
	illusionable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 70,
	targetDistance = 1,
	runHealth = 0,
	healthHidden = false,
	isBlockable = false,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true,
	pet = false
}

monster.light = {
	level = 0,
	color = 0
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Your soul will be mine!", yell = false},
	{text = "MUHAHAHAHA!", yell = false},
	{text = "CHAMEK ATH UTHUL ARAK!", yell = false},
	{text = "I SMELL FEEEEAAAAAR!", yell = false},
	{text = "Your resistance is futile!", yell = false}
}

monster.loot = {
	{id = 2148, chance = 97120, maxCount = 199}, -- gold coin
	{id = 2152, chance = 59460, maxCount = 8}, -- platinum coin
	{id = 2795, chance = 20770, maxCount = 6}, -- fire mushroom
	{id = 8473, chance = 19590, maxCount = 3}, -- ultimate health potion
	{id = 2387, chance = 16450}, -- double axe
	{id = 7590, chance = 14840, maxCount = 3}, -- great mana potion
	{id = 2149, chance = 9640, maxCount = 5}, -- small emerald
	{id = 7368, chance = 5150, maxCount = 5}, -- assassin star
	{id = 2432, chance = 3840}, -- fire axe
	{id = 2151, chance = 3560}, -- talon
	{id = 2176, chance = 2830}, -- orb
	{id = 2393, chance = 2090}, -- giant sword
	{id = 2418, chance = 1490}, -- golden sickle
	{id = 2165, chance = 1380}, -- stealth ring
	{id = 1982, chance = 1290}, -- purple tome
	{id = 2462, chance = 1210}, -- devil helmet
	{id = 2179, chance = 1060}, -- gold ring
	{id = 2171, chance = 740}, -- platinum amulet
	{id = 2396, chance = 700}, -- ice rapier
	{id = 2520, chance = 670}, -- demon shield
	{id = 5954, chance = 510}, -- demon horn
	{id = 2214, chance = 480}, -- ring of healing
	{id = 2470, chance = 420}, -- golden legs
	{id = 2514, chance = 410}, -- mastermind shield
	{id = 2164, chance = 160}, -- might ring
	{id = 7393, chance = 100}, -- demon trophy
	{id = 7382, chance = 80}, -- demonrage sword
	{id = 2472, chance = 70} -- magic plate armor
}

monster.attacks = {
	{name ="melee", interval = 2000, chance = 100, minDamage = 0, maxDamage = -500},
	{name ="combat", interval = 2000, chance = 20, type = COMBAT_FIREDAMAGE, minDamage = -150, maxDamage = -250, range = 7, radius = 7, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true},
	{name ="combat", interval = 2000, chance = 10, type = COMBAT_LIFEDRAIN, minDamage = -300, maxDamage = -480, length = 8, spread = 0, effect = CONST_ME_PURPLEENERGY, target = false},
	{name ="combat", interval = 2000, chance = 10, type = COMBAT_ENERGYDAMAGE, minDamage = -210, maxDamage = -300, range = 1, shootEffect = CONST_ANI_ENERGY, target = true},
	{name ="combat", interval = 2000, chance = 10, type = COMBAT_MANADRAIN, minDamage = -30, maxDamage = -120, range = 7, target = true},
	{name ="firefield", interval = 2000, chance = 10, range = 7, radius = 1, shootEffect = CONST_ANI_FIRE, target = true},
	{name ="speed", interval = 2000, chance = 15, speed = -700, radius = 1, effect = CONST_ME_REDSHIMMER, target = true, duration = 30000}
}

monster.defenses = {
	defense = 55,
	armor = 44,
	{name ="combat", interval = 2000, chance = 15, type = COMBAT_HEALING, minDamage = 80, maxDamage = 250, effect = CONST_ME_MAGIC_BLUE, target = false},
	{name ="speed", interval = 2000, chance = 15, speed = 320, effect = CONST_ME_REDSHIMMER, target = false, duration = 5000}
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 25},
	{type = COMBAT_DEATHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 50},
	{type = COMBAT_EARTHDAMAGE, percent = 40},
	{type = COMBAT_ICEDAMAGE, percent = -12},
	{type = COMBAT_HOLYDAMAGE, percent = -12}
}

monster.immunities = {
	{type = "fire", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "paralyze", condition = true},
	{type = "invisible", condition = true}
}

monster.summons = {
	{name = "fire elemental", chance = 10, interval = 2000, max = 1}
}

mType:register(monster)
