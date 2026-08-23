-- Generated from XML. Source: monsters/orc spearman.xml
return {
  schema = 1,
  name = "Orc Spearman",
  description = "an orc spearman",
  race = "blood",
  experience = 38,
  speed = 48,
  mana_cost = 310,
  health = 105,
  max_health = 105,
  outfit = {
    look_type = 50,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2920,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 19,
      attack = 17,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 15,
      min = -16,
      max = -40,
      range = 7,
      shoot = "spear",
    },
  },
  defenses = {
    armor = 6,
    defense = 12,
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
    { text = "Ugaar!", yell = false },
  },
  loot = {
    { id = 2468, chance = 10000 }, -- studded legs
    { id = 2482, chance = 9000 }, -- studded helmet
    { id = 2389, chance = 23000 }, -- spear
    { id = 2666, chance = 30000 }, -- meat
    { id = 2420, chance = 10000 }, -- machete
    { id = 2148, chance = 22000, count_max = 7 }, -- gold coin
    { id = 2220, chance = 7700 }, -- dirty fur
  },
}
