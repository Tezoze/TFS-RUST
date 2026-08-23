-- Generated from XML. Source: monsters/orc warrior.xml
return {
  schema = 1,
  name = "Orc Warrior",
  description = "an orc warrior",
  race = "blood",
  experience = 50,
  speed = 55,
  mana_cost = 360,
  health = 125,
  max_health = 125,
  outfit = {
    look_type = 7,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2862,
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
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 42,
      attack = 19,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 8,
    defense = 14,
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
    { text = "Grow truk grrrr.", yell = false },
    { text = "Trak grrrr brik.", yell = false },
    { text = "Alk!", yell = false },
  },
  loot = {
    { id = 2512, chance = 18000 }, -- wooden shield
    { id = 2385, chance = 50000 }, -- sabre
    { id = 2411, chance = 100 }, -- poison dagger
    { id = 2666, chance = 15000 }, -- meat
    { id = 2148, chance = 65000, count_max = 15 }, -- gold coin
    { id = 2530, chance = 500 }, -- copper shield
    { id = 2464, chance = 7500 }, -- chain armor
    { id = 2007, chance = 7000 }, -- bottle
  },
}
