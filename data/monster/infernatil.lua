-- Generated from XML. Source: monsters/infernatil.xml
return {
  schema = 1,
  name = "Infernatil",
  description = "",
  race = "fire",
  experience = 30000,
  speed = 210,
  mana_cost = 0,
  health = 110000,
  max_health = 110000,
  outfit = {
    look_type = 35,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2916,
  },
  change_target = { chance = 15 },
  target_strategy = { nearest = 60, weakest = 5, most_damage = 30, random = 5 },
  lose_target = { chance = 15 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 3000,
  },
  attacks = {
    {
      name = "melee",
      skill = 210,
      attack = 260,
      skill_factor = 1000,
      skill_next_level = 50,
      skill_add_count = 5,
    },
    {
      name = "fire",
      delay = 10,
      min = -300,
      max = -1500,
      length = 8,
      spread = 3,
      effect = "firearea",
    },
    {
      name = "fire",
      delay = 8,
      min = -500,
      max = -1000,
      length = 8,
      spread = 0,
      effect = "explosionarea",
    },
    {
      name = "fire",
      delay = 3,
      min = -350,
      max = -850,
      range = 7,
      radius = 7,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
    {
      name = "firefield",
      delay = 30,
      radius = 8,
      target = false,
      effect = "explosionarea",
    },
    {
      name = "fire",
      delay = 2,
      min = -200,
      max = -500,
      radius = 5,
      target = false,
      effect = "fire",
    },
    {
      name = "physical",
      delay = 15,
      min = -250,
      max = -750,
      radius = 6,
      target = false,
      effect = "explosionarea",
    },
    {
      name = "firecondition",
      delay = 15,
      range = 2,
      cycle = 1000,
      min_cycle = 200,
      shoot = "fire",
      effect = "fire",
    },
  },
  defenses = {
    armor = 165,
    defense = 150,
    spells = {
      {
        name = "speed",
        delay = 11,
        duration = 4000,
        speed = 90,
        speed_variation = 5,
        effect = "redshimmer",
      },
      {
        name = "healing",
        delay = 4,
        min = 2000,
        max = 3000,
        effect = "blueshimmer",
      },
      {
        name = "healing",
        delay = 7,
        min = 5000,
        max = 10000,
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
    { text = "BOW TO THE POWER OF THE RUTHLESS SEVEN!", yell = true },
    { text = "THE DAY OF RECKONING IS AT HAND!", yell = true },
    { text = "YOU ALL WILL BURN!", yell = true },
    { text = "ASHES TO ASHES!", yell = true },
  },
  summons = {
    max = 4,
    { name = "Demon", delay = 14, max = 4 },
  },
  loot = {
    { id = 2143, chance = 12500, count_max = 15 }, -- white pearl
    { id = 2188, chance = 2500 }, -- wand of plague
    { id = 3955, chance = 100 }, -- voodoo doll
    { id = 2185, chance = 3500 }, -- volcanic rod
    { id = 2377, chance = 20000 }, -- two handed sword
    { id = 2421, chance = 13500 }, -- thunder hammer
    { id = 2112, chance = 14500 }, -- teddy bear
    { id = 2151, chance = 14000, count_max = 7 }, -- talon
    { id = 2174, chance = 2500 }, -- strange symbol
    { id = 2197, chance = 4000 }, -- stone skin amulet
    { id = 2165, chance = 9500 }, -- stealth ring
    { id = 2182, chance = 3500 }, -- snakebite rod
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
    { id = 2186, chance = 3500 }, -- moonlight rod
    { id = 2178, chance = 4000 }, -- mind stone
    { id = 2164, chance = 5000 }, -- might ring
    { id = 2514, chance = 7500 }, -- mastermind shield
    { id = 2472, chance = 3000 }, -- magic plate armor
    { id = 2162, chance = 11500 }, -- magic light wand
    { id = 2177, chance = 1000 }, -- life crystal
    { id = 2396, chance = 7500 }, -- ice rapier
    { id = 2155, chance = 1500 }, -- green gem
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
    { id = 2195, chance = 4000 }, -- boots of haste
    { id = 2158, chance = 1500 }, -- blue gem
    { id = 2144, chance = 15000, count_max = 15 }, -- black pearl
    { id = 2231, chance = 9000 }, -- big bone
    { id = 2142, chance = 3500 }, -- ancient amulet
  },
}
