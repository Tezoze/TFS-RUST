-- Generated from XML. Source: monsters/ferumbras.xml
return {
  schema = 1,
  name = "Ferumbras",
  description = "",
  race = "venom",
  experience = 9999,
  speed = 155,
  mana_cost = 0,
  health = 28000,
  max_health = 28000,
  outfit = {
    look_type = 130,
    look_head = 57,
    look_body = 113,
    look_legs = 95,
    look_feet = 113,
    corpse = 4240,
  },
  change_target = { chance = 20 },
  target_strategy = { nearest = 60, weakest = 5, most_damage = 30, random = 5 },
  lose_target = { chance = 20 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 2500,
  },
  attacks = {
    {
      name = "melee",
      skill = 165,
      attack = 155,
      skill_factor = 1000,
      skill_next_level = 50,
      skill_add_count = 5,
    },
    {
      name = "lifedrain",
      delay = 12,
      min = -50,
      max = -850,
      length = 8,
      spread = 0,
      effect = "greenspark",
    },
    {
      name = "firecondition",
      delay = 7,
      range = 7,
      radius = 7,
      cycle = 500,
      min_cycle = 200,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
    {
      name = "lifedrain",
      delay = 10,
      min = -200,
      max = -400,
      radius = 6,
      target = false,
      effect = "poff",
    },
    {
      name = "manadrain",
      delay = 13,
      min = -125,
      max = -375,
      radius = 6,
      target = false,
      effect = "redshimmer",
    },
    {
      name = "energycondition",
      delay = 11,
      radius = 6,
      cycle = 650,
      min_cycle = 50,
      target = false,
      effect = "energy",
    },
    {
      name = "poisoncondition",
      delay = 12,
      radius = 6,
      cycle = 400,
      min_cycle = 350,
      target = false,
      effect = "poison",
    },
    {
      name = "manadrain",
      delay = 9,
      min = -350,
      max = -650,
      range = 7,
      effect = "redshimmer",
    },
  },
  defenses = {
    armor = 90,
    defense = 110,
    spells = {
      {
        name = "invisible",
        delay = 46,
        duration = 9000,
        effect = "blueshimmer",
      },
      {
        name = "speed",
        delay = 26,
        duration = 7000,
        speed = 95,
        speed_variation = 5,
        effect = "blueshimmer",
      },
      {
        name = "healing",
        delay = 10,
        min = 1400,
        max = 2600,
        effect = "greenshimmer",
      },
      {
        name = "healing",
        delay = 4,
        min = 600,
        max = 1000,
        effect = "greenshimmer",
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
    { text = "NOONE WILL STOP ME THIS TIME!", yell = true },
    { text = "THE POWER IS MINE!", yell = true },
    { text = "I returned from death and you dream about defeating me?", yell = false },
    { text = "Witness the first seconds of my eternal world domination!", yell = false },
    { text = "The powers of darkness are with me!", yell = false },
    { text = "Even in my weakened state I will crush you all!", yell = false },
    { text = "I came, I see, I will win!", yell = false },
  },
  summons = {
    max = 4,
    { name = "Demon", delay = 9, max = 4 },
  },
  loot = {
    { id = 3026, chance = 12500, count_max = 15 }, -- white pearl
    { id = 3069, chance = 3500 }, -- volcanic rod
    { id = 3002, chance = 100 }, -- voodoo doll
    { id = 3265, chance = 20000 }, -- two handed sword
    { id = 3309, chance = 13500 }, -- thunder hammer
    { id = 2993, chance = 14500 }, -- teddy bear
    { id = 3034, chance = 14000, count_max = 7 }, -- talon
    { id = 3058, chance = 2500 }, -- strange symbol
    { id = 3081, chance = 4000 }, -- stone skin amulet
    { id = 3049, chance = 9500 }, -- stealth ring
    { id = 3029, chance = 13500, count_max = 10 }, -- small sapphire
    { id = 3032, chance = 15500, count_max = 10 }, -- small emerald
    { id = 3028, chance = 9500, count_max = 5 }, -- small diamond
    { id = 3033, chance = 13500, count_max = 20 }, -- small amethyst
    { id = 3324, chance = 5000 }, -- skull staff
    { id = 3290, chance = 15500 }, -- silver dagger
    { id = 3054, chance = 13000 }, -- silver amulet
    { id = 3006, chance = 3500 }, -- ring of the sky
    { id = 3098, chance = 13000 }, -- ring of healing
    { id = 2848, chance = 2600 }, -- purple tome
    { id = 3084, chance = 4500 }, -- protection amulet
    { id = 3055, chance = 4500 }, -- platinum amulet
    { id = 3060, chance = 12000 }, -- orb
    { id = 3062, chance = 4000 }, -- mind stone
    { id = 3048, chance = 5000 }, -- might ring
    { id = 3414, chance = 7500 }, -- mastermind shield
    { id = 3366, chance = 3000 }, -- magic plate armor
    { id = 3046, chance = 11500 }, -- magic light wand
    { id = 3061, chance = 1000 }, -- life crystal
    { id = 3284, chance = 7500 }, -- ice rapier
    { id = 3072, chance = 2500 }, -- wand of plague
    { id = 3038, chance = 1500 }, -- green gem
    { id = 3066, chance = 3500 }, -- snakebite rod
    { id = 3306, chance = 4500 }, -- golden sickle
    { id = 2903, chance = 7500 }, -- golden mug
    { id = 3364, chance = 5000 }, -- golden legs
    { id = 3063, chance = 8000 }, -- gold ring
    { id = 3031, chance = 66600, count_max = 100 }, -- gold coin
    { id = 3031, chance = 77700, count_max = 100 }, -- gold coin
    { id = 3031, chance = 88800, count_max = 100 }, -- gold coin
    { id = 3031, chance = 99900, count_max = 100 }, -- gold coin
    { id = 3281, chance = 12500 }, -- giant sword
    { id = 3320, chance = 17000 }, -- fire axe
    { id = 3051, chance = 13500 }, -- energy ring
    { id = 3322, chance = 4500 }, -- dragon hammer
    { id = 3275, chance = 20000 }, -- double axe
    { id = 3356, chance = 11000 }, -- devil helmet
    { id = 3420, chance = 15500 }, -- demon shield
    { id = 3007, chance = 5500 }, -- crystal ring
    { id = 3008, chance = 1500 }, -- crystal necklace
    { id = 3076, chance = 2500 }, -- crystal ball
    { id = 3070, chance = 3500 }, -- moonlight rod
    { id = 3079, chance = 4000 }, -- boots of haste
    { id = 3041, chance = 1500 }, -- blue gem
    { id = 3027, chance = 15000, count_max = 15 }, -- black pearl
    { id = 3025, chance = 3500 }, -- ancient amulet
  },
}
