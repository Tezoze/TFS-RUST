-- Generated from XML. Source: monsters/elf.xml
return {
  schema = 1,
  name = "Elf",
  description = "an elf",
  race = "blood",
  experience = 42,
  speed = 55,
  mana_cost = 320,
  health = 100,
  max_health = 100,
  outfit = {
    look_type = 62,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2945,
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
      skill = 20,
      attack = 12,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 20,
      min = -15,
      max = -35,
      range = 7,
      shoot = "arrow",
    },
  },
  defenses = {
    armor = 6,
    defense = 12,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Ulathil beia Thratha!", yell = false },
    { text = "Bahaha aka!", yell = false },
    { text = "You are not welcome here.", yell = false },
    { text = "Flee as long as you can.", yell = false },
    { text = "Death to the defilers!", yell = false },
  },
  loot = {
    { id = 2482, chance = 15000 }, -- studded helmet
    { id = 2484, chance = 11000 }, -- studded armor
    { id = 2674, chance = 20000, count_max = 2 }, -- red apple
    { id = 2397, chance = 8000 }, -- longsword
    { id = 2643, chance = 11000 }, -- leather boots
    { id = 2511, chance = 13000 }, -- brass shield
    { id = 2544, chance = 7000, count_max = 3 }, -- arrow
  },
}
