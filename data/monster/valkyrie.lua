-- Generated from XML. Source: monsters/valkyrie.xml
return {
  schema = 1,
  name = "Valkyrie",
  description = "a valkyrie",
  race = "blood",
  experience = 85,
  speed = 48,
  mana_cost = 450,
  health = 190,
  max_health = 190,
  outfit = {
    look_type = 139,
    look_head = 113,
    look_body = 38,
    look_legs = 76,
    look_feet = 96,
    corpse = 3065,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 35,
      attack = 20,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 7,
      min = -28,
      max = -42,
      range = 7,
      shoot = "spear",
    },
  },
  defenses = {
    armor = 12,
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
    { text = "Stand still!", yell = false },
    { text = "One more head for me!", yell = false },
    { text = "Head off!", yell = false },
  },
  loot = {
    { id = 2389, chance = 60000, count_max = 3 }, -- spear
    { id = 2145, chance = 100 }, -- small diamond
    { id = 2229, chance = 80000, count_max = 2 }, -- skull
    { id = 2674, chance = 7500, count_max = 2 }, -- red apple
    { id = 2200, chance = 1100 }, -- protection amulet
    { id = 2463, chance = 800 }, -- plate armor
    { id = 2666, chance = 30000, count_max = 3 }, -- meat
    { id = 2148, chance = 32000, count_max = 12 }, -- gold coin
    { id = 2387, chance = 400 }, -- double axe
    { id = 2379, chance = 25000 }, -- dagger
    { id = 2458, chance = 4000 }, -- chain helmet
    { id = 2464, chance = 10000 }, -- chain armor
  },
}
