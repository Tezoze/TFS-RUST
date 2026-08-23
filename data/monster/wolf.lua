-- Generated from XML. Source: monsters/wolf.xml
return {
  schema = 1,
  name = "Wolf",
  description = "a wolf",
  race = "blood",
  experience = 18,
  speed = 42,
  mana_cost = 255,
  health = 25,
  max_health = 25,
  outfit = {
    look_type = 27,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2826,
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
    run_health = 8,
  },
  attacks = {
    {
      name = "melee",
      skill = 19,
      attack = 12,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 1,
    defense = 4,
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
    { id = 3976, chance = 10000 }, -- worm
    { id = 2666, chance = 50000, count_max = 2 }, -- meat
  },
}
