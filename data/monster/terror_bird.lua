-- Generated from XML. Source: monsters/terror bird.xml
return {
  schema = 1,
  name = "Terror Bird",
  description = "a terror bird",
  race = "blood",
  experience = 150,
  speed = 66,
  mana_cost = 490,
  health = 300,
  max_health = 300,
  outfit = {
    look_type = 218,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4317,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 0, most_damage = 30, random = 0 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 46,
      attack = 37,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 13,
    defense = 21,
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
    { text = "CRAAAHHH!", yell = false },
    { text = "Gruuuh Gruuuh.", yell = false },
    { text = "Carrah Carrah!", yell = false },
  },
  loot = {
    { id = 3976, chance = 20000, count_max = 3 }, -- worm
    { id = 2666, chance = 50000, count_max = 2 }, -- meat
    { id = 2148, chance = 40000, count_max = 20 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 10 }, -- gold coin
    { id = 3970, chance = 100 }, -- feather headdress
  },
}
