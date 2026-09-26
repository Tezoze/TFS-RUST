-- Generated from XML. Source: monsters/goblin.xml
return {
  schema = 1,
  name = "Goblin",
  description = "a goblin",
  race = "blood",
  experience = 25,
  speed = 20,
  mana_cost = 290,
  health = 50,
  max_health = 50,
  outfit = {
    look_type = 61,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4121,
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
    run_health = 15,
  },
  attacks = {
    {
      name = "melee",
      skill = 15,
      attack = 10,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 12,
      min = -15,
      max = -25,
      range = 7,
      shoot = "smallstone",
    },
  },
  defenses = {
    armor = 6,
    defense = 8,
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
    { text = "Me have him!", yell = false },
    { text = "Zig Zag! Gobo attack!", yell = false },
    { text = "Help! Goblinkiller!", yell = false },
    { text = "Bugga! Bugga!", yell = false },
    { text = "Me green, me mean!", yell = false },
  },
  loot = {
    { id = 1781, chance = 30000, count_max = 3 }, -- small stone
    { id = 3462, chance = 10000 }, -- small axe
    { id = 3294, chance = 9000 }, -- short sword
    { id = 3120, chance = 7000 }, -- moldy cheese
    { id = 3355, chance = 10000 }, -- leather helmet
    { id = 3361, chance = 7500 }, -- leather armor
    { id = 3031, chance = 50000, count_max = 9 }, -- gold coin
    { id = 3578, chance = 13000 }, -- fish
    { id = 3267, chance = 18000 }, -- dagger
    { id = 3337, chance = 5000 }, -- bone club
    { id = 3115, chance = 12000 }, -- bone
  },
}
