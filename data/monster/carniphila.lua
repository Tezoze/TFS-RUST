-- Generated from XML. Source: monsters/carniphila.xml
return {
  schema = 1,
  name = "Carniphila",
  description = "a carniphila",
  race = "venom",
  experience = 150,
  speed = 15,
  mana_cost = 490,
  health = 255,
  max_health = 255,
  outfit = {
    look_type = 120,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4345,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 0, most_damage = 30, random = 0 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 40,
      attack = 40,
      poison_cycles = 95,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "poison",
      delay = 9,
      min = -40,
      max = -130,
      radius = 3,
      target = false,
      effect = "poison",
    },
    {
      name = "speed",
      delay = 3,
      range = 7,
      duration = 30000,
      speed = -135,
      speed_variation = 25,
      shoot = "poison",
      effect = "greenspark",
    },
    {
      name = "poison",
      delay = 4,
      min = -60,
      max = -90,
      range = 7,
      shoot = "poison",
      effect = "greenspark",
    },
  },
  defenses = {
    armor = 26,
    defense = 20,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = false,
    invisible = true,
  },
  loot = {
    { id = 3738, chance = 500 }, -- sling herb
    { id = 3738, chance = 500 }, -- sling herb
    { id = 3740, chance = 1000 }, -- shadow herb
    { id = 3577, chance = 70000, count_max = 2 }, -- meat
    { id = 3582, chance = 40000 }, -- ham
    { id = 3661, chance = 500 }, -- grave flower
    { id = 3031, chance = 40000, count_max = 10 }, -- gold coin
    { id = 3031, chance = 80000, count_max = 30 }, -- gold coin
    { id = 3728, chance = 8000 }, -- dark mushroom
  },
}
