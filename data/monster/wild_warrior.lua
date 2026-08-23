-- Generated from XML. Source: monsters/wild warrior.xml
return {
  schema = 1,
  name = "Wild Warrior",
  description = "a wild warrior",
  race = "blood",
  experience = 60,
  speed = 55,
  mana_cost = 420,
  health = 135,
  max_health = 135,
  outfit = {
    look_type = 131,
    look_head = 38,
    look_body = 38,
    look_legs = 38,
    look_feet = 38,
    corpse = 3058,
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
      skill = 46,
      attack = 16,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 9,
    defense = 18,
    spells = {
      {
        name = "speed",
        delay = 17,
        duration = 2000,
        speed = 40,
        speed_variation = 20,
        effect = "redshimmer",
      },
    },
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
    { text = "An enemy!", yell = false },
    { text = "Gimme your money!", yell = false },
  },
  loot = {
    { id = 2391, chance = 100 }, -- war hammer
    { id = 2509, chance = 1000 }, -- steel shield
    { id = 2666, chance = 40000 }, -- meat
    { id = 2398, chance = 10000 }, -- mace
    { id = 2649, chance = 15000 }, -- leather legs
    { id = 2459, chance = 500 }, -- iron helmet
    { id = 2148, chance = 15000, count_max = 10 }, -- gold coin
    { id = 2148, chance = 40000, count_max = 20 }, -- gold coin
    { id = 2110, chance = 500 }, -- doll
    { id = 2458, chance = 5000 }, -- chain helmet
    { id = 2511, chance = 17000 }, -- brass shield
    { id = 2465, chance = 2500 }, -- brass armor
    { id = 2386, chance = 30000 }, -- axe
  },
}
