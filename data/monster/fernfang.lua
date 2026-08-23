-- Generated from XML. Source: monsters/fernfang.xml
return {
  schema = 1,
  name = "Fernfang",
  description = "",
  race = "blood",
  experience = 400,
  speed = 95,
  mana_cost = 0,
  health = 400,
  max_health = 400,
  outfit = {
    look_type = 206,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3058,
  },
  change_target = { chance = 7 },
  target_strategy = { nearest = 70, weakest = 10, most_damage = 20, random = 0 },
  lose_target = { chance = 7 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 4,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 50,
      attack = 40,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 10,
      min = -140,
      max = -180,
      range = 7,
      shoot = "death",
    },
    {
      name = "energy",
      delay = 8,
      min = -70,
      max = -90,
      range = 7,
      shoot = "energy",
      effect = "energy",
    },
    {
      name = "manadrain",
      delay = 9,
      min = -25,
      max = -45,
      range = 7,
    },
  },
  defenses = {
    armor = 25,
    defense = 50,
    spells = {
      {
        name = "outfit",
        delay = 21,
        duration = 14000,
        monster = "War Wolf",
        effect = "blueshimmer",
      },
      {
        name = "speed",
        delay = 15,
        duration = 10000,
        speed = 85,
        speed_variation = 5,
        effect = "redshimmer",
      },
      {
        name = "healing",
        delay = 7,
        min = 90,
        max = 120,
        effect = "blueshimmer",
      },
    },
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
  voices = {
    { text = "You desecrated this place!", yell = false },
    { text = "I will cleanse this isle!", yell = false },
    { text = "Grrrrrrr", yell = false },
    { text = "Yoooohhuuuu!", yell = true },
  },
  summons = {
    max = 4,
    { name = "War Wolf", delay = 8, max = 4 },
  },
  loot = {
    { id = 2154, chance = 400 }, -- yellow gem
    { id = 2129, chance = 10000 }, -- wolf tooth chain
    { id = 2800, chance = 9000 }, -- star herb
    { id = 2401, chance = 11000 }, -- staff
    { id = 2401, chance = 11000 }, -- staff
    { id = 2802, chance = 7000 }, -- sling herb
    { id = 2642, chance = 9000 }, -- sandals
    { id = 2166, chance = 500 }, -- power ring
    { id = 2044, chance = 10000 }, -- lamp
    { id = 2177, chance = 2000 }, -- life crystal
    { id = 2652, chance = 9000 }, -- green tunic
    { id = 2747, chance = 9000 }, -- grave flower
    { id = 2148, chance = 15000, count_max = 18 }, -- gold coin
    { id = 2220, chance = 7700 }, -- dirty fur
    { id = 2015, chance = 9000 }, -- brown flask
    { id = 2689, chance = 14000 }, -- bread
    { id = 2032, chance = 6500 }, -- bowl
    { id = 2260, chance = 18000 }, -- blank rune
  },
}
