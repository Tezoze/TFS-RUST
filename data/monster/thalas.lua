-- Generated from XML. Source: monsters/thalas.xml
return {
  schema = 1,
  name = "Thalas",
  description = "",
  race = "undead",
  experience = 2950,
  speed = 90,
  mana_cost = 0,
  health = 4100,
  max_health = 4100,
  outfit = {
    look_type = 89,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3016,
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
      skill = 110,
      attack = 65,
      poison_cycles = 700,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "poison",
      delay = 6,
      min = -250,
      max = -550,
      length = 8,
      spread = 3,
      effect = "poison",
    },
    {
      name = "speed",
      delay = 7,
      radius = 5,
      duration = 12000,
      speed = -80,
      speed_variation = 20,
      target = false,
      effect = "poison",
    },
    {
      name = "poisoncondition",
      delay = 7,
      radius = 5,
      cycle = 550,
      min_cycle = 150,
      target = false,
      effect = "poison",
    },
    {
      name = "speed",
      delay = 17,
      range = 7,
      duration = 50000,
      speed = -100,
      speed_variation = 10,
      effect = "redshimmer",
    },
    {
      name = "poison",
      delay = 4,
      min = -300,
      max = -650,
      range = 7,
      shoot = "poison",
      effect = "poison",
    },
    {
      name = "lifedrain",
      delay = 16,
      min = -400,
      max = -900,
      range = 1,
    },
  },
  defenses = {
    armor = 40,
    defense = 60,
    spells = {
      {
        name = "healing",
        delay = 5,
        min = 150,
        max = 450,
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
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "You will become a feast for my maggots.", yell = false },
    { text = "Death and decay!", yell = false },
    { text = "Death awaits you.", yell = false },
    { text = "Your precious life will be wasted.", yell = false },
    { text = "Green is my favourite color.", yell = false },
    { text = "Flesssh to dussst!", yell = false },
  },
  summons = {
    max = 8,
    { name = "Slime", delay = 12, max = 8 },
  },
  loot = {
    { id = 2169, chance = 5000 }, -- time ring
    { id = 2149, chance = 10000, count_max = 3 }, -- small emerald
    { id = 2409, chance = 2000 }, -- serpent sword
    { id = 2411, chance = 20000 }, -- poison dagger
    { id = 2155, chance = 1000 }, -- green gem
    { id = 2148, chance = 35000, count_max = 95 }, -- gold coin
    { id = 2148, chance = 50000, count_max = 85 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 80 }, -- gold coin
    { id = 2451, chance = 1500 }, -- djinn blade
    { id = 2351, chance = 100000 }, -- cobrafang dagger
  },
}
