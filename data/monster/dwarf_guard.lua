-- Generated from XML. Source: monsters/dwarf guard.xml
return {
  schema = 1,
  name = "Dwarf Guard",
  description = "a dwarf guard",
  race = "blood",
  experience = 165,
  speed = 63,
  mana_cost = 650,
  health = 245,
  max_health = 245,
  outfit = {
    look_type = 70,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2983,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 0, most_damage = 20, random = 10 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 55,
      attack = 39,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 15,
    defense = 37,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Hail Durin!", yell = false },
  },
  loot = {
    { id = 2787, chance = 55000, count_max = 2 }, -- white mushroom
    { id = 2457, chance = 2000 }, -- steel helmet
    { id = 2150, chance = 100 }, -- small amethyst
    { id = 2483, chance = 10000 }, -- scale armor
    { id = 2643, chance = 40000 }, -- leather boots
    { id = 2148, chance = 50000, count_max = 30 }, -- gold coin
    { id = 2387, chance = 600 }, -- double axe
    { id = 2513, chance = 7500 }, -- battle shield
    { id = 2417, chance = 4000 }, -- battle hammer
    { id = 2208, chance = 200 }, -- axe ring
  },
}
