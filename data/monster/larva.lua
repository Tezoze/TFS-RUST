-- Generated from XML. Source: monsters/larva.xml
return {
  schema = 1,
  name = "Larva",
  description = "a larva",
  race = "venom",
  experience = 44,
  speed = 22,
  mana_cost = 355,
  health = 70,
  max_health = 70,
  outfit = {
    look_type = 82,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3010,
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
      skill = 30,
      attack = 20,
      poison_cycles = 15,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 5,
    defense = 11,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = true,
    invisible = false,
  },
  loot = {
    { id = 2666, chance = 30000 }, -- meat
    { id = 2148, chance = 35000, count_max = 10 }, -- gold coin
  },
}
