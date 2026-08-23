-- Generated from XML. Source: monsters/scarab.xml
return {
  schema = 1,
  name = "Scarab",
  description = "a scarab",
  race = "venom",
  experience = 120,
  speed = 40,
  mana_cost = 395,
  health = 320,
  max_health = 320,
  outfit = {
    look_type = 83,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3013,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 80,
  },
  attacks = {
    {
      name = "melee",
      skill = 42,
      attack = 25,
      skill_factor = 1500,
      skill_next_level = 50,
      skill_add_count = 1,
    },
    {
      name = "poison",
      delay = 6,
      min = -15,
      max = -25,
      range = 1,
      radius = 1,
      target = true,
      effect = "poison",
    },
    {
      name = "poisonfield",
      delay = 5,
      radius = 1,
      target = false,
      effect = "poff",
    },
  },
  defenses = {
    armor = 21,
    defense = 26,
    spells = {
      {
        name = "speed",
        delay = 20,
        duration = 4000,
        speed = 45,
        speed_variation = 5,
        effect = "redshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = true,
    invisible = false,
  },
  loot = {
    { id = 2149, chance = 300 }, -- small emerald
    { id = 2150, chance = 500 }, -- small amethyst
    { id = 2159, chance = 100 }, -- scarab coin
    { id = 2159, chance = 1000 }, -- scarab coin
    { id = 2666, chance = 54000, count_max = 2 }, -- meat
    { id = 2442, chance = 500 }, -- heavy machete
    { id = 2148, chance = 44500, count_max = 40 }, -- gold coin
    { id = 2148, chance = 70500, count_max = 12 }, -- gold coin
    { id = 2439, chance = 300 }, -- daramanian mace
    { id = 2544, chance = 5000, count_max = 3 }, -- arrow
  },
}
