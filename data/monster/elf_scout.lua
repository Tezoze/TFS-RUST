-- Generated from XML. Source: monsters/elf scout.xml
return {
  schema = 1,
  name = "Elf Scout",
  description = "an elf scout",
  race = "blood",
  experience = 75,
  speed = 70,
  mana_cost = 360,
  health = 160,
  max_health = 160,
  outfit = {
    look_type = 64,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2981,
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
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 25,
      attack = 18,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 4,
      min = -30,
      max = -60,
      range = 7,
      shoot = "arrow",
    },
  },
  defenses = {
    armor = 7,
    defense = 18,
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
    { text = "Tha'shi Ab'Dendriel!", yell = false },
    { text = "Feel the sting of my arrows!", yell = false },
    { text = "Thy blood will quench the soil's thirst!", yell = false },
    { text = "Evicor guide my arrow.", yell = false },
    { text = "Your existence will end here!", yell = false },
  },
  loot = {
    { id = 2031, chance = 14000 }, -- waterskin
    { id = 2482, chance = 8000 }, -- studded helmet
    { id = 2484, chance = 12000 }, -- studded armor
    { id = 2642, chance = 10000 }, -- sandals
    { id = 2545, chance = 15000, count_max = 3 }, -- poison arrow
    { id = 2397, chance = 6000 }, -- longsword
    { id = 2681, chance = 18000 }, -- grapes
    { id = 2148, chance = 30000, count_max = 5 }, -- gold coin
    { id = 2456, chance = 4000 }, -- bow
    { id = 2544, chance = 30000, count_max = 12 }, -- arrow
  },
}
