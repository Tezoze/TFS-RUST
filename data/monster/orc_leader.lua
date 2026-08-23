-- Generated from XML. Source: monsters/orc leader.xml
return {
  schema = 1,
  name = "Orc Leader",
  description = "an orc leader",
  race = "blood",
  experience = 270,
  speed = 75,
  mana_cost = 640,
  health = 450,
  max_health = 450,
  outfit = {
    look_type = 59,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2938,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 15, most_damage = 15, random = 0 },
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
      skill = 52,
      attack = 48,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 6,
      min = -50,
      max = -70,
      range = 7,
      shoot = "throwingknife",
    },
  },
  defenses = {
    armor = 20,
    defense = 45,
  },
  immunities = {
    fire = true,
    energy = false,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Ulderek futgyr human!", yell = false },
  },
  loot = {
    { id = 2475, chance = 100 }, -- warrior helmet
    { id = 2410, chance = 10000, count_max = 4 }, -- throwing knife
    { id = 2207, chance = 4000 }, -- sword ring
    { id = 2419, chance = 12000 }, -- scimitar
    { id = 2510, chance = 10000 }, -- plate shield
    { id = 2647, chance = 400 }, -- plate legs
    { id = 2463, chance = 1500 }, -- plate armor
    { id = 2666, chance = 15000, count_max = 2 }, -- meat
    { id = 2397, chance = 8000 }, -- longsword
    { id = 2148, chance = 28000, count_max = 35 }, -- gold coin
    { id = 2667, chance = 30000 }, -- fish
    { id = 2379, chance = 23000 }, -- dagger
    { id = 2413, chance = 800 }, -- broadsword
    { id = 2478, chance = 2500 }, -- brass legs
    { id = 1988, chance = 20000 }, -- backpack
  },
}
