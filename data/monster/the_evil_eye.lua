-- Generated from XML. Source: monsters/the evil eye.xml
return {
  schema = 1,
  name = "The Evil Eye",
  description = "",
  race = "blood",
  experience = 500,
  speed = 55,
  mana_cost = 0,
  health = 1100,
  max_health = 1100,
  outfit = {
    look_type = 210,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3052,
  },
  change_target = { chance = 30 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 30 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
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
      skill = 65,
      attack = 24,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "manadrain",
      delay = 12,
      min = -150,
      max = -250,
      length = 8,
      spread = 3,
      effect = "bluebubble",
    },
    {
      name = "lifedrain",
      delay = 18,
      min = -75,
      max = -85,
      length = 8,
      spread = 3,
      effect = "redshimmer",
    },
    {
      name = "poison",
      delay = 13,
      min = -35,
      max = -85,
      length = 8,
      spread = 3,
      effect = "greenbubble",
    },
    {
      name = "speed",
      delay = 10,
      range = 7,
      duration = 20000,
      speed = -95,
      speed_variation = 15,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 9,
      min = -110,
      max = -130,
      range = 7,
      effect = "redshimmer",
    },
    {
      name = "poison",
      delay = 7,
      min = -40,
      max = -120,
      range = 7,
      shoot = "poison",
    },
    {
      name = "physical",
      delay = 6,
      min = -135,
      max = -175,
      range = 7,
      shoot = "death",
      effect = "mortarea",
    },
    {
      name = "fire",
      delay = 8,
      min = -85,
      max = -115,
      range = 7,
      shoot = "fire",
    },
    {
      name = "energy",
      delay = 7,
      min = -60,
      max = -130,
      range = 7,
      shoot = "energy",
    },
  },
  defenses = {
    armor = 19,
    defense = 35,
    spells = {
      {
        name = "healing",
        delay = 12,
        min = 1,
        max = 219,
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
    { text = "653768764!", yell = false },
    { text = "Let me take a look at you!", yell = false },
    { text = "Inferior creatures, bow before my power!", yell = false },
    { text = "659978 54764!", yell = false },
  },
  summons = {
    max = 6,
    { name = "Ghost", delay = 9, max = 6 },
    { name = "Demon Skeleton", delay = 8, max = 6 },
  },
  loot = {
    { id = 2512, chance = 1500 }, -- wooden shield
    { id = 2377, chance = 4000 }, -- two handed sword
    { id = 2509, chance = 4000 }, -- steel shield
    { id = 2175, chance = 5000 }, -- spellbook
    { id = 2394, chance = 7000 }, -- morning star
    { id = 2397, chance = 9000 }, -- longsword
    { id = 2148, chance = 70000, count_max = 40 }, -- gold coin
    { id = 2148, chance = 80000, count_max = 32 }, -- gold coin
    { id = 2148, chance = 90000, count_max = 24 }, -- gold coin
    { id = 2518, chance = 200 }, -- beholder shield
  },
}
