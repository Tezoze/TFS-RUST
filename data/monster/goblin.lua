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
    corpse = 2940,
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
    { id = 1294, chance = 30000, count_max = 3 }, -- small stone
    { id = 2559, chance = 10000 }, -- small axe
    { id = 2406, chance = 9000 }, -- short sword
    { id = 2235, chance = 7000 }, -- moldy cheese
    { id = 2461, chance = 10000 }, -- leather helmet
    { id = 2467, chance = 7500 }, -- leather armor
    { id = 2148, chance = 50000, count_max = 9 }, -- gold coin
    { id = 2667, chance = 13000 }, -- fish
    { id = 2379, chance = 18000 }, -- dagger
    { id = 2449, chance = 5000 }, -- bone club
    { id = 2230, chance = 12000 }, -- bone
  },
}
