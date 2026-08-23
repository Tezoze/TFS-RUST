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
    corpse = 3058,
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
    { id = 2143, chance = 12500, count_max = 15 }, -- white pearl
    { id = 2185, chance = 3500 }, -- volcanic rod
    { id = 3955, chance = 100 }, -- voodoo doll
    { id = 2377, chance = 20000 }, -- two handed sword
    { id = 2421, chance = 13500 }, -- thunder hammer
    { id = 2112, chance = 14500 }, -- teddy bear
    { id = 2151, chance = 14000, count_max = 7 }, -- talon
    { id = 2174, chance = 2500 }, -- strange symbol
    { id = 2197, chance = 4000 }, -- stone skin amulet
    { id = 2165, chance = 9500 }, -- stealth ring
    { id = 2146, chance = 13500, count_max = 10 }, -- small sapphire
    { id = 2149, chance = 15500, count_max = 10 }, -- small emerald
    { id = 2145, chance = 9500, count_max = 5 }, -- small diamond
    { id = 2150, chance = 13500, count_max = 20 }, -- small amethyst
    { id = 2436, chance = 5000 }, -- skull staff
    { id = 2402, chance = 15500 }, -- silver dagger
    { id = 2170, chance = 13000 }, -- silver amulet
    { id = 2123, chance = 3500 }, -- ring of the sky
    { id = 2214, chance = 13000 }, -- ring of healing
    { id = 1982, chance = 2600 }, -- purple tome
    { id = 2200, chance = 4500 }, -- protection amulet
    { id = 2171, chance = 4500 }, -- platinum amulet
    { id = 2176, chance = 12000 }, -- orb
    { id = 2178, chance = 4000 }, -- mind stone
    { id = 2164, chance = 5000 }, -- might ring
    { id = 2514, chance = 7500 }, -- mastermind shield
    { id = 2472, chance = 3000 }, -- magic plate armor
    { id = 2162, chance = 11500 }, -- magic light wand
    { id = 2177, chance = 1000 }, -- life crystal
    { id = 2396, chance = 7500 }, -- ice rapier
    { id = 2188, chance = 2500 }, -- wand of plague
    { id = 2155, chance = 1500 }, -- green gem
    { id = 2182, chance = 3500 }, -- snakebite rod
    { id = 2418, chance = 4500 }, -- golden sickle
    { id = 2033, chance = 7500 }, -- golden mug
    { id = 2470, chance = 5000 }, -- golden legs
    { id = 2179, chance = 8000 }, -- gold ring
    { id = 2148, chance = 66600, count_max = 100 }, -- gold coin
    { id = 2148, chance = 77700, count_max = 100 }, -- gold coin
    { id = 2148, chance = 88800, count_max = 100 }, -- gold coin
    { id = 2148, chance = 99900, count_max = 100 }, -- gold coin
    { id = 2393, chance = 12500 }, -- giant sword
    { id = 2432, chance = 17000 }, -- fire axe
    { id = 2167, chance = 13500 }, -- energy ring
    { id = 2434, chance = 4500 }, -- dragon hammer
    { id = 2387, chance = 20000 }, -- double axe
    { id = 2462, chance = 11000 }, -- devil helmet
    { id = 2520, chance = 15500 }, -- demon shield
    { id = 2124, chance = 5500 }, -- crystal ring
    { id = 2125, chance = 1500 }, -- crystal necklace
    { id = 2192, chance = 2500 }, -- crystal ball
    { id = 2186, chance = 3500 }, -- moonlight rod
    { id = 2195, chance = 4000 }, -- boots of haste
    { id = 2158, chance = 1500 }, -- blue gem
    { id = 2144, chance = 15000, count_max = 15 }, -- black pearl
    { id = 2142, chance = 3500 }, -- ancient amulet
  },
}
