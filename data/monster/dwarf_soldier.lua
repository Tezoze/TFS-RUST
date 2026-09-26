-- Generated from XML. Source: monsters/dwarf soldier.xml
return {
  schema = 1,
  name = "Dwarf Soldier",
  description = "a dwarf soldier",
  race = "blood",
  experience = 70,
  speed = 48,
  mana_cost = 360,
  health = 135,
  max_health = 135,
  outfit = {
    look_type = 71,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4166,
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
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 42,
      attack = 21,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 8,
      min = -20,
      max = -40,
      range = 7,
      shoot = "bolt",
    },
  },
  defenses = {
    armor = 9,
    defense = 20,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  voices = {
    { text = "Hail Durin!", yell = false },
  },
  loot = {
    { id = 3723, chance = 40000, count_max = 2 }, -- white mushroom
    { id = 3375, chance = 12000 }, -- soldier helmet
    { id = 3457, chance = 10000 }, -- shovel
    { id = 3031, chance = 35000, count_max = 12 }, -- gold coin
    { id = 3425, chance = 5000 }, -- dwarven shield
    { id = 3349, chance = 4000 }, -- crossbow
    { id = 3358, chance = 9000 }, -- chain armor
    { id = 3446, chance = 40000, count_max = 4 }, -- bolt
    { id = 3266, chance = 2500 }, -- battle axe
    { id = 3092, chance = 100 }, -- axe ring
  },
}
