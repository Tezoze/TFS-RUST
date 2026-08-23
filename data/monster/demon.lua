-- Generated from XML. Source: monsters/demon.xml
return {
  schema = 1,
  name = "Demon",
  description = "a demon",
  race = "fire",
  experience = 6000,
  speed = 80,
  mana_cost = 0,
  health = 8200,
  max_health = 8200,
  outfit = {
    look_type = 35,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2916,
  },
  change_target = { chance = 10 },
  target_strategy = { nearest = 70, weakest = 10, most_damage = 10, random = 10 },
  lose_target = { chance = 10 },
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
      skill = 120,
      attack = 80,
      skill_factor = 1000,
      skill_next_level = 50,
      skill_add_count = 5,
    },
    {
      name = "energy",
      delay = 10,
      min = -300,
      max = -420,
      length = 8,
      spread = 0,
      effect = "energy",
    },
    {
      name = "firefield",
      delay = 7,
      range = 7,
      radius = 1,
      target = true,
      shoot = "fire",
    },
    {
      name = "fire",
      delay = 3,
      min = -110,
      max = -200,
      range = 7,
      radius = 7,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
    {
      name = "manadrain",
      delay = 8,
      min = -40,
      max = -100,
      range = 7,
    },
  },
  defenses = {
    armor = 40,
    defense = 65,
    spells = {
      {
        name = "healing",
        delay = 7,
        min = 90,
        max = 150,
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
    { text = "MUHAHAHAHA!", yell = true },
    { text = "I SMELL FEEEEEAAAR!", yell = true },
    { text = "CHAMEK ATH UTHUL ARAK!", yell = true },
    { text = "Your resistance is futile!", yell = false },
    { text = "Your soul will be mine!", yell = true },
  },
  summons = {
    max = 1,
    { name = "Fire Elemental", delay = 12, max = 1 },
  },
  loot = {
    { id = 2151, chance = 3500 }, -- talon
    { id = 2165, chance = 1400 }, -- stealth ring
    { id = 2149, chance = 11000 }, -- small emerald
    { id = 2214, chance = 500 }, -- ring of healing
    { id = 1982, chance = 1300 }, -- purple tome
    { id = 2171, chance = 700 }, -- platinum amulet
    { id = 2176, chance = 3000 }, -- orb
    { id = 2164, chance = 200 }, -- might ring
    { id = 2514, chance = 500 }, -- mastermind shield
    { id = 2472, chance = 100 }, -- magic plate armor
    { id = 2396, chance = 600 }, -- ice rapier
    { id = 2418, chance = 1500 }, -- golden sickle
    { id = 2470, chance = 400 }, -- golden legs
    { id = 2179, chance = 1100 }, -- gold ring
    { id = 2148, chance = 40000, count_max = 100 }, -- gold coin
    { id = 2148, chance = 50000, count_max = 100 }, -- gold coin
    { id = 2148, chance = 60000, count_max = 100 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 100 }, -- gold coin
    { id = 2393, chance = 2000 }, -- giant sword
    { id = 2795, chance = 20000, count_max = 6 }, -- fire mushroom
    { id = 2432, chance = 4000 }, -- fire axe
    { id = 2387, chance = 20000 }, -- double axe
    { id = 2462, chance = 1200 }, -- devil helmet
    { id = 2520, chance = 700 }, -- demon shield
  },
}
