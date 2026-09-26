-- Generated from XML. Source: monsters/bandit.xml
return {
  schema = 1,
  name = "Bandit",
  description = "a bandit",
  race = "blood",
  experience = 65,
  speed = 50,
  mana_cost = 450,
  health = 245,
  max_health = 245,
  outfit = {
    look_type = 129,
    look_head = 58,
    look_body = 40,
    look_legs = 24,
    look_feet = 95,
    corpse = 4240,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 25,
  },
  attacks = {
    {
      name = "melee",
      skill = 37,
      attack = 20,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 11,
    defense = 17,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  voices = {
    { text = "Your money or your life!", yell = false },
    { text = "Hand me your purse!", yell = false },
  },
  loot = {
    { id = 3279, chance = 100 }, -- war hammer
    { id = 3577, chance = 10000 }, -- meat
    { id = 3286, chance = 10000 }, -- mace
    { id = 3559, chance = 15000 }, -- leather legs
    { id = 3353, chance = 500 }, -- iron helmet
    { id = 3031, chance = 15000, count_max = 10 }, -- gold coin
    { id = 3031, chance = 40000, count_max = 20 }, -- gold coin
    { id = 3352, chance = 5000 }, -- chain helmet
    { id = 3411, chance = 17000 }, -- brass shield
    { id = 3359, chance = 2500 }, -- brass armor
    { id = 3274, chance = 30000 }, -- axe
  },
}
