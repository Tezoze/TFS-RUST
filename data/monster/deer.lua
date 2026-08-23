-- Generated from XML. Source: monsters/deer.xml
return {
  schema = 1,
  name = "Deer",
  description = "a deer",
  race = "blood",
  experience = 0,
  speed = 58,
  mana_cost = 260,
  health = 25,
  max_health = 25,
  outfit = {
    look_type = 31,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2835,
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
      attack = 2,
      skill_factor = 1100,
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
    { id = 2666, chance = 80000, count_max = 3 }, -- meat
    { id = 2671, chance = 45000 }, -- ham
  },
}
