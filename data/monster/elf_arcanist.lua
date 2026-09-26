-- Generated from XML. Source: monsters/elf arcanist.xml
return {
  schema = 1,
  name = "Elf Arcanist",
  description = "an elf arcanist",
  race = "blood",
  experience = 175,
  speed = 70,
  mana_cost = 0,
  health = 220,
  max_health = 220,
  outfit = {
    look_type = 63,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4160,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
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
      skill = 25,
      attack = 20,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 9,
      min = -60,
      max = -80,
      range = 7,
      shoot = "death",
    },
    {
      name = "energy",
      delay = 12,
      min = -30,
      max = -50,
      range = 7,
      shoot = "energy",
      effect = "energy",
    },
    {
      name = "physical",
      delay = 11,
      min = -15,
      max = -45,
      range = 7,
      shoot = "arrow",
    },
  },
  defenses = {
    armor = 15,
    defense = 20,
    spells = {
      {
        name = "healing",
        delay = 5,
        min = 42,
        max = 68,
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
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Feel my wrath!", yell = false },
    { text = "For the Daughter of the Stars!", yell = false },
    { text = "I'll bring balance upon you!", yell = false },
    { text = "Tha'shi Cenath!", yell = false },
    { text = "Vihil Ealuel!", yell = false },
  },
  loot = {
    { id = 3037, chance = 200 }, -- yellow gem
    { id = 3073, chance = 1000 }, -- wand of cosmic energy
    { id = 3289, chance = 11000 }, -- staff
    { id = 3738, chance = 5000 }, -- sling herb
    { id = 2815, chance = 30000 }, -- scroll
    { id = 3551, chance = 13000 }, -- sandals
    { id = 3593, chance = 22000 }, -- melon
    { id = 3061, chance = 1000 }, -- life crystal
    { id = 3509, chance = 9000 }, -- inkwell
    { id = 3563, chance = 7000 }, -- green tunic
    { id = 3661, chance = 7000 }, -- grave flower
    { id = 3082, chance = 2000 }, -- elven amulet
    { id = 2917, chance = 22000 }, -- candlestick
    { id = 3600, chance = 14000 }, -- bread
    { id = 2902, chance = 5500 }, -- bowl
    { id = 3147, chance = 18000 }, -- blank rune
    { id = 3447, chance = 6000, count_max = 3 }, -- arrow
  },
}
