-- Generated from XML. Source: monsters/assassin.xml
return {
  schema = 1,
  name = "Assassin",
  description = "an assassin",
  race = "blood",
  experience = 105,
  speed = 72,
  mana_cost = 450,
  health = 175,
  max_health = 175,
  outfit = {
    look_type = 129,
    look_head = 95,
    look_body = 95,
    look_legs = 95,
    look_feet = 95,
    corpse = 4240,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 20, most_damage = 10, random = 0 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 45,
      attack = 45,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "drunk",
      delay = 11,
      range = 6,
      duration = 3000,
      drunkness = 60,
      shoot = "poison",
      effect = "poison",
    },
    {
      name = "physical",
      delay = 8,
      min = -28,
      max = -38,
      range = 7,
      shoot = "throwingstar",
    },
  },
  defenses = {
    armor = 17,
    defense = 40,
    spells = {
      {
        name = "invisible",
        delay = 12,
        duration = 2000,
        effect = "blueshimmer",
      },
      {
        name = "speed",
        delay = 12,
        duration = 3000,
        speed = 60,
        speed_variation = 5,
        effect = "redshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Die!", yell = false },
    { text = "Feel the hand of death!", yell = false },
    { text = "You are on my deathlist!", yell = false },
  },
  loot = {
    { id = 2920, chance = 30000, count_max = 2 }, -- torch
    { id = 3409, chance = 1000 }, -- steel shield
    { id = 3351, chance = 3000 }, -- steel helmet
    { id = 3028, chance = 200 }, -- small diamond
    { id = 3410, chance = 2000 }, -- plate shield
    { id = 3404, chance = 500 }, -- leopard armor
    { id = 3291, chance = 10000 }, -- knife
    { id = 3405, chance = 200 }, -- horseman helmet
    { id = 3031, chance = 15000, count_max = 10 }, -- gold coin
    { id = 3031, chance = 80000, count_max = 40 }, -- gold coin
    { id = 3292, chance = 4000 }, -- combat knife
    { id = 3413, chance = 1500 }, -- battle shield
  },
}
