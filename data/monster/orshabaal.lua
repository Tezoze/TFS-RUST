-- Generated from XML. Source: monsters/orshabaal.xml
return {
  schema = 1,
  name = "Orshabaal",
  description = "",
  race = "fire",
  experience = 9999,
  speed = 150,
  mana_cost = 0,
  health = 22500,
  max_health = 22500,
  outfit = {
    look_type = 201,
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
    run_health = 2500,
  },
  attacks = {
    {
      name = "melee",
      skill = 190,
      attack = 199,
      skill_factor = 1000,
      skill_next_level = 50,
      skill_add_count = 5,
    },
    {
      name = "energy",
      delay = 7,
      min = -500,
      max = -850,
      length = 8,
      spread = 0,
      effect = "energy",
    },
    {
      name = "firefield",
      delay = 11,
      range = 7,
      radius = 4,
      target = true,
      shoot = "fire",
    },
    {
      name = "fire",
      delay = 3,
      min = -310,
      max = -600,
      range = 7,
      radius = 7,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
    {
      name = "manadrain",
      delay = 17,
      min = -150,
      max = -350,
      radius = 5,
      target = false,
      effect = "poison",
    },
    {
      name = "manadrain",
      delay = 8,
      min = -300,
      max = -600,
      range = 7,
    },
  },
  defenses = {
    armor = 90,
    defense = 111,
    spells = {
      {
        name = "speed",
        delay = 21,
        duration = 7000,
        speed = 95,
        speed_variation = 5,
        effect = "redshimmer",
      },
      {
        name = "healing",
        delay = 6,
        min = 600,
        max = 1000,
        effect = "blueshimmer",
      },
      {
        name = "healing",
        delay = 12,
        min = 1500,
        max = 2500,
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
    { text = "PRAISED BE MY MASTERS, THE RUTHLESS SEVEN!", yell = true },
    { text = "YOU ARE DOOMED!", yell = true },
    { text = "ORSHABAAL IS BACK!", yell = true },
    { text = "Be prepared for the day my masters will come for you!", yell = false },
    { text = "SOULS FOR ORSHABAAL!", yell = true },
  },
  summons = {
    max = 4,
    { name = "Demon", delay = 10, max = 4 },
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
