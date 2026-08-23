-- Generated from XML. Source: monsters/gazer.xml
return {
  schema = 1,
  name = "Gazer",
  description = "a gazer",
  race = "blood",
  experience = 90,
  speed = 30,
  mana_cost = 0,
  health = 120,
  max_health = 120,
  outfit = {
    look_type = 109,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3049,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 25,
      attack = 9,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "manadrain",
      delay = 7,
      min = -5,
      max = -15,
      range = 7,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 9,
      min = -25,
      max = -35,
      range = 7,
      effect = "redshimmer",
    },
  },
  defenses = {
    armor = 4,
    defense = 15,
    spells = {
      {
        name = "speed",
        delay = 11,
        duration = 4000,
        speed = 99,
        speed_variation = 1,
        effect = "redshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = true,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Mommy!?", yell = false },
    { text = "Buuuuhaaaahhaaaaa!", yell = false },
    { text = "Me need mana!", yell = false },
  },
  loot = {
    { id = 2512, chance = 3000 }, -- wooden shield
    { id = 2148, chance = 70000, count_max = 10 }, -- gold coin
    { id = 2148, chance = 80000, count_max = 8 }, -- gold coin
    { id = 2148, chance = 90000, count_max = 6 }, -- gold coin
  },
}
