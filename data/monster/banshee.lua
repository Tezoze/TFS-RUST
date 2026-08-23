-- Generated from XML. Source: monsters/banshee.xml
return {
  schema = 1,
  name = "Banshee",
  description = "a banshee",
  race = "undead",
  experience = 900,
  speed = 70,
  mana_cost = 0,
  health = 1000,
  max_health = 1000,
  outfit = {
    look_type = 78,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2998,
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
    run_health = 500,
  },
  attacks = {
    {
      name = "melee",
      skill = 45,
      attack = 30,
      poison_cycles = 65,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "lifedrain",
      delay = 3,
      min = -120,
      max = -200,
      radius = 4,
      target = false,
      effect = "rednote",
    },
    {
      name = "speed",
      delay = 10,
      range = 7,
      duration = 20000,
      speed = -90,
      speed_variation = 30,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 30,
      min = -50,
      max = -350,
      range = 1,
    },
  },
  defenses = {
    armor = 25,
    defense = 65,
    spells = {
      {
        name = "healing",
        delay = 4,
        min = 113,
        max = 187,
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
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "Are you ready to rock?", yell = false },
    { text = "That's what I call easy listening!", yell = false },
    { text = "Let the music play!", yell = false },
    { text = "I will mourn your death!", yell = false },
    { text = "IIIIEEEeeeeeehhhHHHHH!", yell = true },
    { text = "Dance for me your dance of death!", yell = false },
    { text = "Feel my gentle kiss of death.", yell = false },
  },
  loot = {
    { id = 2143, chance = 1000 }, -- white pearl
    { id = 2121, chance = 500 }, -- wedding ring
    { id = 2197, chance = 800 }, -- stone skin amulet
    { id = 2175, chance = 500 }, -- spellbook
    { id = 2657, chance = 60000 }, -- simple dress
    { id = 2134, chance = 1500 }, -- silver brooch
    { id = 2170, chance = 9000 }, -- silver amulet
    { id = 2214, chance = 800 }, -- ring of healing
    { id = 2655, chance = 100 }, -- red robe
    { id = 2411, chance = 1500 }, -- poison dagger
    { id = 2560, chance = 7000 }, -- mirror
    { id = 2071, chance = 1000 }, -- lyre
    { id = 2177, chance = 100 }, -- life crystal
    { id = 2148, chance = 30000, count_max = 80 }, -- gold coin
    { id = 2237, chance = 19900 }, -- dirty cape
    { id = 2124, chance = 100 }, -- crystal ring
    { id = 2047, chance = 70000 }, -- candlestick
    { id = 2656, chance = 600 }, -- blue robe
    { id = 2144, chance = 2000 }, -- black pearl
  },
}
