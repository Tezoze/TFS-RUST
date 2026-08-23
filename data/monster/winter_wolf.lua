-- Generated from XML. Source: monsters/winter wolf.xml
return {
  schema = 1,
  name = "Winter Wolf",
  description = "a winter wolf",
  race = "blood",
  experience = 20,
  speed = 45,
  mana_cost = 260,
  health = 30,
  max_health = 30,
  outfit = {
    look_type = 52,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2924,
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
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 21,
      attack = 13,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 2,
    defense = 6,
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
    { id = 2666, chance = 30000, count_max = 2 }, -- meat
  },
}
