-- Generated from XML. Source: monsters/rahemos.xml
return {
  schema = 1,
  name = "Rahemos",
  description = "",
  race = "undead",
  experience = 3100,
  speed = 100,
  mana_cost = 0,
  health = 3700,
  max_health = 3700,
  outfit = {
    look_type = 88,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3034,
  },
  change_target = { chance = 3 },
  target_strategy = { nearest = 80, weakest = 10, most_damage = 10, random = 0 },
  lose_target = { chance = 3 },
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
      skill = 55,
      attack = 40,
      poison_cycles = 65,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "outfit",
      delay = 7,
      range = 7,
      duration = 12000,
      monster = "Pig",
      effect = "blueshimmer",
    },
    {
      name = "drunk",
      delay = 13,
      range = 7,
      duration = 50000,
      drunkness = 60,
      shoot = "energy",
      effect = "teleport",
    },
    {
      name = "speed",
      delay = 8,
      range = 7,
      duration = 50000,
      speed = -100,
      speed_variation = 10,
      effect = "redshimmer",
    },
    {
      name = "physical",
      delay = 5,
      min = -200,
      max = -600,
      range = 7,
      shoot = "death",
      effect = "mortarea",
    },
    {
      name = "energy",
      delay = 5,
      min = -200,
      max = -600,
      range = 7,
      shoot = "energy",
      effect = "energy",
    },
    {
      name = "lifedrain",
      delay = 15,
      min = -50,
      max = -750,
      range = 1,
    },
  },
  defenses = {
    armor = 40,
    defense = 65,
    spells = {
      {
        name = "outfit",
        delay = 20,
        duration = 4000,
        monster = "Demon",
        effect = "blueshimmer",
      },
      {
        name = "healing",
        delay = 5,
        min = 200,
        max = 500,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = true,
    energy = true,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "It's a kind of magic.", yell = false },
    { text = "Abrah Kadabrah!", yell = false },
    { text = "Nothing hidden in my warpings.", yell = false },
    { text = "It's not a trick, it's Rahemos.", yell = false },
    { text = "Meet my dear friend from hell.", yell = false },
    { text = "I will make you believe in magic.", yell = false },
  },
  summons = {
    max = 1,
    { name = "Demon", delay = 9, max = 1 },
  },
  loot = {
    { id = 2153, chance = 1000 }, -- violet gem
    { id = 2447, chance = 100 }, -- twin axe
    { id = 2150, chance = 10000, count_max = 3 }, -- small amethyst
    { id = 2214, chance = 5000 }, -- ring of healing
    { id = 2176, chance = 500 }, -- orb
    { id = 2662, chance = 2000 }, -- magician hat
    { id = 2148, chance = 35000, count_max = 95 }, -- gold coin
    { id = 2148, chance = 50000, count_max = 85 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 80 }, -- gold coin
    { id = 2184, chance = 100 }, -- crystal wand
    { id = 2348, chance = 100000 }, -- ancient rune
  },
}
