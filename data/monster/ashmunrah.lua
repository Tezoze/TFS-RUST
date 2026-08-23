-- Generated from XML. Source: monsters/ashmunrah.xml
return {
  schema = 1,
  name = "Ashmunrah",
  description = "",
  race = "undead",
  experience = 3100,
  speed = 175,
  mana_cost = 0,
  health = 5000,
  max_health = 5000,
  outfit = {
    look_type = 91,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3034,
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
      skill = 150,
      attack = 74,
      poison_cycles = 165,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "lifedrain",
      delay = 8,
      min = -150,
      max = -550,
      length = 8,
      spread = 3,
      effect = "yellowbubble",
    },
    {
      name = "speed",
      delay = 4,
      range = 7,
      duration = 50000,
      speed = -100,
      speed_variation = 10,
      effect = "redshimmer",
    },
    {
      name = "physical",
      delay = 6,
      min = -250,
      max = -750,
      range = 7,
      shoot = "energy",
      effect = "mortarea",
    },
    {
      name = "poison",
      delay = 9,
      min = -300,
      max = -500,
      range = 7,
      shoot = "poison",
      effect = "poison",
    },
    {
      name = "lifedrain",
      delay = 15,
      min = -400,
      max = -700,
      range = 1,
    },
  },
  defenses = {
    armor = 74,
    defense = 74,
    spells = {
      {
        name = "outfit",
        delay = 35,
        duration = 6000,
        monster = "Ancient Scarab",
        effect = "blueshimmer",
      },
      {
        name = "invisible",
        delay = 15,
        duration = 2000,
        effect = "blueshimmer",
      },
      {
        name = "healing",
        delay = 5,
        min = 200,
        max = 400,
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
    { text = "I might be trapped but not without power.", yell = false },
    { text = "Ahhhh all those long years.", yell = false },
    { text = "Ages come, ages go. Asmumrah remains.", yell = false },
    { text = "My traitorous son has sent thee.", yell = false },
    { text = "No mortal or undead will steal my secrets.", yell = false },
    { text = "You will be history soon.", yell = false },
    { text = "Come to me, my allys and underlings.", yell = false },
  },
  summons = {
    max = 4,
    { name = "Green Djinn", delay = 9, max = 4 },
    { name = "Ancient Scarab", delay = 7, max = 2 },
  },
  loot = {
    { id = 2134, chance = 4000 }, -- silver brooch
    { id = 2164, chance = 5000 }, -- might ring
    { id = 2140, chance = 100 }, -- holy scarab
    { id = 2444, chance = 100 }, -- hammer of wrath
    { id = 2148, chance = 35000, count_max = 95 }, -- gold coin
    { id = 2148, chance = 50000, count_max = 85 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 80 }, -- gold coin
    { id = 2148, chance = 40000, count_max = 90 }, -- gold coin
    { id = 2487, chance = 500 }, -- crown armor
  },
}
