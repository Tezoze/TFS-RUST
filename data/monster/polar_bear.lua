-- Generated from XML. Source: monsters/polar bear.xml
return {
  schema = 1,
  name = "Polar Bear",
  description = "a polar bear",
  race = "blood",
  experience = 28,
  speed = 38,
  mana_cost = 315,
  health = 85,
  max_health = 85,
  outfit = {
    look_type = 42,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2893,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 5,
  },
  attacks = {
    {
      name = "melee",
      skill = 19,
      attack = 18,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 7,
    defense = 10,
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
    { text = "Grrrrrr", yell = false },
    { text = "GROARRR!", yell = true },
  },
  loot = {
    { id = 2666, chance = 70000, count_max = 4 }, -- meat
    { id = 2671, chance = 50000, count_max = 2 }, -- ham
  },
}
