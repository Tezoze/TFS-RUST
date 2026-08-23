-- Generated from XML. Source: monsters/pig.xml
return {
  schema = 1,
  name = "Pig",
  description = "a pig",
  race = "blood",
  experience = 0,
  speed = 17,
  mana_cost = 255,
  health = 25,
  max_health = 25,
  outfit = {
    look_type = 60,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2935,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = false,
    summonable = true,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 25,
  },
  attacks = {
    {
      name = "melee",
      skill = 10,
      attack = 0,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 2,
    defense = 2,
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
  loot = {
    { id = 2666, chance = 65000, count_max = 4 }, -- meat
  },
}
