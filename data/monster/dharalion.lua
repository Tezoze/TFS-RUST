-- Generated from XML. Source: monsters/dharalion.xml
return {
  schema = 1,
  name = "Dharalion",
  description = "",
  race = "blood",
  experience = 380,
  speed = 72,
  mana_cost = 0,
  health = 390,
  max_health = 390,
  outfit = {
    look_type = 203,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2979,
  },
  change_target = { chance = 7 },
  target_strategy = { nearest = 10, weakest = 10, most_damage = 20, random = 60 },
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
      skill = 30,
      attack = 28,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 10,
      min = -130,
      max = -150,
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
      delay = 7,
      min = -30,
      max = -60,
      range = 7,
    },
  },
  defenses = {
    armor = 15,
    defense = 39,
    spells = {
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
        delay = 5,
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
    { text = "You desecrated this temple!", yell = false },
    { text = "Noone will stop my ascension!", yell = false },
    { text = "My powers are divine!", yell = false },
    { text = "Muahahaha!", yell = false },
  },
  summons = {
    max = 2,
    { name = "Demon Skeleton", delay = 18, max = 2 },
  },
  loot = {
    { id = 2154, chance = 400 }, -- yellow gem
    { id = 2401, chance = 11000 }, -- staff
    { id = 2802, chance = 7000 }, -- sling herb
    { id = 1949, chance = 30000 }, -- scroll
    { id = 2642, chance = 9000 }, -- sandals
    { id = 2682, chance = 20000 }, -- melon
    { id = 2177, chance = 1500 }, -- life crystal
    { id = 2600, chance = 9000 }, -- inkwell
    { id = 2652, chance = 9000 }, -- green tunic
    { id = 2747, chance = 9000 }, -- grave flower
    { id = 2198, chance = 2000 }, -- elven amulet
    { id = 2047, chance = 9000 }, -- candlestick
    { id = 2689, chance = 14000 }, -- bread
    { id = 2032, chance = 6500 }, -- bowl
    { id = 2260, chance = 18000 }, -- blank rune
  },
}
