-- Generated from XML. Source: monsters/grorlam.xml
return {
  schema = 1,
  name = "Grorlam",
  description = "",
  race = "undead",
  experience = 1600,
  speed = 100,
  mana_cost = 590,
  health = 2700,
  max_health = 2700,
  outfit = {
    look_type = 205,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4133,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
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
      skill = 75,
      attack = 60,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 7,
      min = -150,
      max = -200,
      range = 7,
      shoot = "largerock",
    },
  },
  defenses = {
    armor = 55,
    defense = 35,
    spells = {
      {
        name = "speed",
        delay = 18,
        duration = 6000,
        speed = 95,
        speed_variation = 5,
        effect = "redshimmer",
      },
      {
        name = "healing",
        delay = 4,
        min = 100,
        max = 150,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = true,
    energy = false,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = false,
    paralyze = true,
    invisible = false,
  },
  loot = {
    { id = 3409, chance = 7000 }, -- steel shield
    { id = 3554, chance = 500 }, -- steel boots
    { id = 1781, chance = 13000, count_max = 4 }, -- small stone
    { id = 3033, chance = 6500, count_max = 2 }, -- small amethyst
    { id = 3377, chance = 5000 }, -- scale armor
    { id = 3039, chance = 500 }, -- red gem
    { id = 3050, chance = 5500 }, -- power ring
    { id = 3456, chance = 6000 }, -- pick
    { id = 3031, chance = 16000, count_max = 15 }, -- gold coin
    { id = 3007, chance = 200 }, -- crystal ring
  },
}
