-- Generated from XML. Source: monsters/bug.xml
return {
  schema = 1,
  name = "Bug",
  description = "a bug",
  race = "venom",
  experience = 18,
  speed = 40,
  mana_cost = 250,
  health = 29,
  max_health = 29,
  outfit = {
    look_type = 45,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2902,
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
      skill = 23,
      attack = 9,
      skill_factor = 1000,
      skill_next_level = 50,
      skill_add_count = 2,
    },
  },
  defenses = {
    armor = 2,
    defense = 3,
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
    { id = 2148, chance = 35000, count_max = 6 }, -- gold coin
    { id = 2679, chance = 3000, count_max = 3 }, -- cherry
  },
}
