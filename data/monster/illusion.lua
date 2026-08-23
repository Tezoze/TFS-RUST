-- Generated from XML. Source: monsters/illusion.xml
return {
  schema = 1,
  name = "Illusion",
  title = "Demon",
  description = "a demon",
  race = "blood",
  experience = 25,
  speed = 20,
  mana_cost = 0,
  health = 50,
  max_health = 50,
  outfit = {
    look_type = 107,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2940,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
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
      delay = 8,
      min = -15,
      max = -25,
      length = 8,
      spread = 0,
      effect = "energy",
    },
    {
      name = "physical",
      delay = 8,
      min = -15,
      max = -25,
      range = 7,
      radius = 2,
      target = true,
      shoot = "fire",
      effect = "firearea",
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
    outfit = true,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "MUHAHAHAHA!", yell = true },
    { text = "I SMELL FEEEEEAAAR!", yell = true },
    { text = "CHAMEK ATH UTHUL ARAK!", yell = true },
    { text = "Your resistance is futile!", yell = false },
    { text = "Your soul will be mine!", yell = true },
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
    { id = 2230, chance = 12000 }, -- bone
  },
}
