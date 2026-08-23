-- Generated from XML. Source: monsters/amazon.xml
return {
  schema = 1,
  name = "Amazon",
  description = "an amazon",
  race = "blood",
  experience = 60,
  speed = 46,
  mana_cost = 390,
  health = 110,
  max_health = 110,
  outfit = {
    look_type = 137,
    look_head = 113,
    look_body = 120,
    look_legs = 95,
    look_feet = 115,
    corpse = 3065,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 24,
      attack = 16,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 10,
      min = -20,
      max = -30,
      range = 7,
      shoot = "throwingknife",
    },
  },
  defenses = {
    armor = 11,
    defense = 11,
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
    { text = "Yeeee ha!", yell = false },
    { text = "Your head will be mine!", yell = false },
  },
  loot = {
    { id = 2050, chance = 5000 }, -- torch
    { id = 2526, chance = 5000 }, -- studded shield
    { id = 2147, chance = 100 }, -- small ruby
    { id = 2229, chance = 80000, count_max = 2 }, -- skull
    { id = 2385, chance = 23000 }, -- sabre
    { id = 2467, chance = 50000 }, -- leather armor
    { id = 2148, chance = 40000, count_max = 10 }, -- gold coin
    { id = 2379, chance = 80000 }, -- dagger
    { id = 2125, chance = 200 }, -- crystal necklace
    { id = 2691, chance = 30000 }, -- brown bread
  },
}
