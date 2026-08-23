-- Generated from XML. Source: monsters/necromancer.xml
return {
  schema = 1,
  name = "Necromancer",
  description = "a necromancer",
  race = "blood",
  experience = 580,
  speed = 54,
  mana_cost = 0,
  health = 580,
  max_health = 580,
  outfit = {
    look_type = 9,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3058,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 4,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 30,
      attack = 40,
      poison_cycles = 90,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "poison",
      delay = 6,
      min = -35,
      max = -95,
      range = 7,
      shoot = "poison",
      effect = "poison",
    },
    {
      name = "lifedrain",
      delay = 5,
      min = -60,
      max = -100,
      range = 1,
      effect = "redspark",
    },
  },
  defenses = {
    armor = 50,
    defense = 40,
    spells = {
      {
        name = "healing",
        delay = 4,
        min = 42,
        max = 68,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Your corpse will be mine!", yell = false },
    { text = "Taste the sweetness of death!", yell = false },
  },
  summons = {
    max = 2,
    { name = "Mummy", delay = 8, max = 1 },
    { name = "Ghost", delay = 7, max = 1 },
    { name = "Ghoul", delay = 6, max = 2 },
  },
  loot = {
    { id = 2436, chance = 100 }, -- skull staff
    { id = 2406, chance = 15000 }, -- short sword
    { id = 2483, chance = 10000 }, -- scale armor
    { id = 2663, chance = 500 }, -- mystic turban
    { id = 2796, chance = 1500 }, -- green mushroom
    { id = 2148, chance = 30000, count_max = 90 }, -- gold coin
    { id = 2423, chance = 1000 }, -- clerical mace
    { id = 2195, chance = 200 }, -- boots of haste
  },
}
