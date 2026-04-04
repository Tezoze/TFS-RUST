local mType = Game.createMonsterType("Training Monk")
local monster = {}
monster.description = "a training monk"
monster.experience = 0
monster.outfit = {
	lookType = 57,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}
monster.health = 2500
monster.maxHealth = 2500
monster.race = "blood"
monster.speed = 250
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 95,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{
		text = "Train hard or go home!",
		yell = false
	},
	{
		text = "Feel the power of faith!",
		yell = false
	},
	{
		text = "Repent and train!",
		yell = false
	}
}

monster.loot = {
	{ id = 2148, chance = 80000, maxCount = 10 }, -- gold coins (common, low value)
	{ id = 2689, chance = 50000 } -- bread (filler)
}

monster.attacks = {
	{ name = "melee", interval = 2000, minDamage = 0, maxDamage = -2 } -- Low, consistent melee for shielding training
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{ name = "combat", interval = 2000, chance = 40, minDamage = 120, maxDamage = 200, type = COMBAT_HEALING, effect = CONST_ME_MAGIC_BLUE } -- Frequent strong heal
}

monster.elements = {
	{ type = COMBAT_PHYSICALDAMAGE, percent = -10 }, -- Slightly weak to phys (easy to kill)
	{ type = COMBAT_HOLYDAMAGE, percent = 20 },
	{ type = COMBAT_DEATHDAMAGE, percent = 20 }
}

monster.immunities = {
	{ type = "paralyze", combat = false, condition = true },
	{ type = "outfit", false },
	{ type = "drunk", false },
	{ type = "invisible", combat = false, condition = true }
}

mType:register(monster)
