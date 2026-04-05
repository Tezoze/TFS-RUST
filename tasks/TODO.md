Core Engine

 Game loop / scheduler (dispatcher + scheduler tasks)
 Connection management (login server, game server, connection lifecycle)
 XTEA encryption/decryption
 Packet framing (length-prefixed, checksums)
 RSA handshake

Map & World

 OTBM map loader
 OTB item loader
 Tile system (ground, items, creatures, flags)
 Pathfinding (A*)
 Line of sight
 Sectors / map chunks

Movement

 Walk (N/S/E/W)
 Diagonal walk
 Auto walk / pathfinding walk
 Speed system (base speed, haste, paralyze)
 Walk delay / exhaustion
 Push creatures
 Teleport
 Stair / ladder climbing (floor change)
 Swimming / field movement penalties
 Mounting

Creatures

 Creature base (id, name, health, position, direction)
 Player creature
 NPC creature
 Monster creature
 Creature spawn system
 Respawn timers
 Creature light
 Outfit / look
 Skulls
 Shield (party)
 Creature visibility (viewport, known creatures)

Combat

 Melee attack
 Distance attack
 Magic attack (runes, spells)
 Combat formulas (attack, defense, armor)
 Critical hits
 Hit animations / magic effects
 Projectile animations
 Death / death penalty
 Corpse / loot
 Skull system (white, yellow, red)
 Unjustified kills / frag system
 War system / guild war
 PvP zones (no pvp, pvp, hardcore pvp)

Spells & Magic

 Spell registry / loader
 Instant spells
 Rune spells
 Conjure spells
 House spells
 Spell cooldowns / exhaustion
 Mana cost
 Vocation restrictions

Items

 Item base (id, count, attributes)
 Stackable items
 Containers (open, close, move items inside)
 Depots
 Equipment slots (armor, weapon, shield, etc.)
 Item decay
 Item use (on self, on target, on tile)
 Fluid containers (liquids)
 Doors (open, close, lock)
 Beds
 Teleport items
 Mailbox
 Readable items (books, signs)
 Runes
 Ammunition
 Keys

Player

 Character creation
 Vocations
 Skills (fist, club, sword, axe, distance, shield, fishing, magic level)
 Skill advancement formula
 Experience / level system
 Health / mana / capacity
 Stamina
 Soul points
 Conditions (poison, fire, energy, haste, paralyze, drunk, etc.)
 Save / load player (DB)
 Inventory management
 Depot management
 Premium account
 Prey system (if applicable)

NPCs

 NPC dialogue system
 Keyword matching
 Trade / shop (buy, sell)
 NPC idle / focus behavior
 NPC walkback

Monsters

 Monster loader (XML)
 Monster AI (target selection, flee, idle walk)
 Monster spells
 Monster loot tables
 Summons
 Convincing

Houses

 House loader
 House ownership
 Access lists (guest, subowner)
 Door permissions
 Rent system
 House tile protection
 Kick from house

Economy & Trade

 Player to player trade
 NPC shop
 Market (if 1.4 supports it)
 Gold / platinum / crystal coin stacking

Social

 Chat channels (local, world, trade, help, guild, party)
 Private messages
 VIP list (online/offline notification)
 Party system (shared exp, loot)
 Guild system

Game Systems

 Day/night cycle (world light)
 Weather
 Global events / game events
 Tasks / bestiary
 Quests / quest log
 Highscores

Lua Scripting

 Lua runtime integration
 Script bindings (creature, item, tile, player)
 Event hooks (onLogin, onDeath, onUse, onStep, etc.)
 Creaturescripts
 Talkactions
 Globalevents
 Movements
 Actions
 Spells scripted via Lua
 NPC scripts

Admin / GM

 GM commands
 Banning (account, IP, character)
 Teleport / summon
 Set outfit / item / skill
 Broadcast
 Kickall

Persistence

 MySQL / SQLite integration
 Player save (position, stats, skills, inventory)
 House save
 Guild save
 Account system
 IP bans / account bans