-- Generated from XML. Source: monsters/troll.xml
return {
  schema = 1,
  name = "Troll",
  description = "a troll",
  race = "blood",
  experience = 20,
  speed = 23,
  mana_cost = 290,
  health = 50,
  max_health = 50,
  outfit = {
    look_type = 15,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2806,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 15,
  },
  attacks = {
    {
      name = "melee",
      skill = 15,
      attack = 10,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 6,
    defense = 8,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  voices = {
    { text = "Grrrr", yell = false },
    { text = "Groar", yell = false },
    { text = "Gruntz!", yell = false },
    { text = "Hmmm, bugs.", yell = false },
    { text = "Hmmm, dogs.", yell = false },
  },
  loot = {
    { id = 2512, chance = 15000 }, -- wooden shield
    { id = 2448, chance = 5000 }, -- studded club
    { id = 2389, chance = 20000 }, -- spear
    { id = 2170, chance = 100 }, -- silver amulet
    { id = 2120, chance = 8000 }, -- rope
    { id = 2666, chance = 15000 }, -- meat
    { id = 2461, chance = 10000 }, -- leather helmet
    { id = 2643, chance = 10000 }, -- leather boots
    { id = 2380, chance = 18000 }, -- hand axe
    { id = 2148, chance = 60000, count_max = 10 }, -- gold coin
  },
}
