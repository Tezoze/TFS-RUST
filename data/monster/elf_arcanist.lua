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
    corpse = 2979,
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
    { id = 2154, chance = 200 }, -- yellow gem
    { id = 2189, chance = 1000 }, -- wand of cosmic energy
    { id = 2401, chance = 11000 }, -- staff
    { id = 2802, chance = 5000 }, -- sling herb
    { id = 1949, chance = 30000 }, -- scroll
    { id = 2642, chance = 13000 }, -- sandals
    { id = 2682, chance = 22000 }, -- melon
    { id = 2177, chance = 1000 }, -- life crystal
    { id = 2600, chance = 9000 }, -- inkwell
    { id = 2652, chance = 7000 }, -- green tunic
    { id = 2747, chance = 7000 }, -- grave flower
    { id = 2198, chance = 2000 }, -- elven amulet
    { id = 2047, chance = 22000 }, -- candlestick
    { id = 2689, chance = 14000 }, -- bread
    { id = 2032, chance = 5500 }, -- bowl
    { id = 2260, chance = 18000 }, -- blank rune
    { id = 2544, chance = 6000, count_max = 3 }, -- arrow
  },
}
