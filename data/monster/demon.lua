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
    corpse = 4097,
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
    { id = 3034, chance = 3500 }, -- talon
    { id = 3049, chance = 1400 }, -- stealth ring
    { id = 3032, chance = 11000 }, -- small emerald
    { id = 3098, chance = 500 }, -- ring of healing
    { id = 2848, chance = 1300 }, -- purple tome
    { id = 3055, chance = 700 }, -- platinum amulet
    { id = 3060, chance = 3000 }, -- orb
    { id = 3048, chance = 200 }, -- might ring
    { id = 3414, chance = 500 }, -- mastermind shield
    { id = 3366, chance = 100 }, -- magic plate armor
    { id = 3284, chance = 600 }, -- ice rapier
    { id = 3306, chance = 1500 }, -- golden sickle
    { id = 3364, chance = 400 }, -- golden legs
    { id = 3063, chance = 1100 }, -- gold ring
    { id = 3031, chance = 40000, count_max = 100 }, -- gold coin
    { id = 3031, chance = 50000, count_max = 100 }, -- gold coin
    { id = 3031, chance = 60000, count_max = 100 }, -- gold coin
    { id = 3031, chance = 70000, count_max = 100 }, -- gold coin
    { id = 3281, chance = 2000 }, -- giant sword
    { id = 3731, chance = 20000, count_max = 6 }, -- fire mushroom
    { id = 3320, chance = 4000 }, -- fire axe
    { id = 3275, chance = 20000 }, -- double axe
    { id = 3356, chance = 1200 }, -- devil helmet
    { id = 3420, chance = 700 }, -- demon shield
  },
}
