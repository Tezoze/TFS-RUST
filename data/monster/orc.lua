-- Generated from XML. Source: monsters/orc.xml
return {
  schema = 1,
  name = "Orc",
  description = "an orc",
  race = "blood",
  experience = 25,
  speed = 35,
  mana_cost = 300,
  health = 70,
  max_health = 70,
  outfit = {
    look_type = 5,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2820,
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
      skill = 22,
      attack = 13,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 4,
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
    { text = "Grak brrretz!", yell = false },
    { text = "Grow truk grrrrr.", yell = false },
    { text = "Prek tars, dekklep zurk.", yell = false },
  },
  loot = {
    { id = 2526, chance = 10000 }, -- studded shield
    { id = 2482, chance = 9000 }, -- studded helmet
    { id = 2484, chance = 12000 }, -- studded armor
    { id = 2385, chance = 6000 }, -- sabre
    { id = 2666, chance = 20000 }, -- meat
    { id = 2148, chance = 85000, count_max = 8 }, -- gold coin
    { id = 2386, chance = 8000 }, -- axe
  },
}
