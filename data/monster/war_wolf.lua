-- Generated from XML. Source: monsters/war wolf.xml
return {
  schema = 1,
  name = "War Wolf",
  description = "a war wolf",
  race = "blood",
  experience = 55,
  speed = 92,
  mana_cost = 420,
  health = 140,
  max_health = 140,
  outfit = {
    look_type = 3,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2969,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 39,
      attack = 24,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 8,
    defense = 16,
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
    { text = "Grrrrrrr", yell = false },
    { text = "Yoooohhuuuu!", yell = true },
  },
  loot = {
    { id = 2666, chance = 70000, count_max = 4 }, -- meat
    { id = 2671, chance = 40000, count_max = 2 }, -- ham
  },
}
