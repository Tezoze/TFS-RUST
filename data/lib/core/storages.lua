--[[
Reserved storage ranges:
- 300000 to 301000+ reserved for achievements
- 20000 to 21000+ reserved for achievement progress
- 10000000 to 20000000 reserved for outfits and mounts on source
]] --
PlayerStorageKeys = {
	annihilatorReward = 30015,
	promotion = 30018,
	delayLargeSeaShell = 30019,
	firstRod = 30020,
	delayWallMirror = 88023,
	madSheepSummon = 88024,
	crateUsable = 88025,
	achievementsBase = 300000,
	achievementsCounter = 20000
}

GlobalStorageKeys = {
	-- Yakchal storage
	Yakchal = 30001,

	-- Elemental Sphere storage
	ElementalSphere = {
		BossRoom = 30006,
		Machine1 = 30002,
		Machine2 = 30003,
		Machine3 = 30004,
		Machine4 = 30005,
		MachineGemCount = 30007,
		QuestLine = 30008
	},

	-- Pits of Inferno storage
	PitsOfInfernoLevers = 30334,

	-- The Ancient Tombs storage
	TheAncientTombs = {
		ThalasSwitchesGlobalStorage = 30045,
		DiprathSwitchesGlobalStorage = 30046,
		AshmunrahSwitchesGlobalStorage = 30047
	},

	-- In Service of Yalahar storage
	InServiceOfYalahar = {
		LastFight = 30066,
		WarGolemsMachine1 = 30067,
		WarGolemsMachine2 = 30068
	},

	-- Wrath of the Emperor storage
	WrathOfTheEmperor = {
		Bosses = {
			Fury = 88017,
			Wrath = 30850,
			Scorn = 30851,
			Spite = 30852
		},
		Light01 = 30853,
		Light02 = 30854,
		Light03 = 30855,
		Light04 = 88018
	},

	-- Fury Gates storage
	FuryGates = 100,

	-- Their Masters Voice storage
	TheirMastersVoice = {
		CurrentServantWave = 984,
		ServantsKilled = 985
	},

	-- Yakchal storage
	Yakchal = 987,

	-- Naginata Stone storage
	NaginataStone = 50058,

	-- Experience Boost storage
	ExpBoost = 51052,

	-- Sword of Fury storage
	SwordOfFury = 5635,

	-- Experience Display Mode storage
	XpDisplayMode = 5634
}

-- Storage table for quest and NPC related storage values
Storage = {
	firstMageWeapon = 30022,

	-- Blood Brothers Quest storage
	BloodBrothers = {
		Questline = 45230,
		Mission01 = 45231,
		Mission02 = 45232,
		Mission03 = 45233,
		Mission04 = 45234,
		Mission05 = 45235,
		Mission06 = 45236,
		Mission07 = 45237,
		Mission08 = 45238,
		Mission09 = 45239,
		Mission10 = 45240,
		BorekthKill = 45241,
		LersatioKill = 45242,
		MarzielKill = 45243,
		ArtheiKill = 45244,
		GarlicCookieCount = 45245,
		LisanderSuspect = 45246,
		SerafinSuspect = 45247,
		OrtheusSuspect = 45248,
		VengothAccess = 45249,
		RewardSelection = 45250,
		MarisSuspect = 45251,
		ArmeniusSuspect = 45252,
		HisTrueFace = 45253,
		VengothSpots = {
			BottomlessPit = 45254,     -- Pitch Black Gap
			BoneCircle = 45255,        -- 6 Bone Totems
			HauntedRuin = 45256,       -- Building with A Wandering Soul
			LonelyGrave = 45257,       -- Grave with inscription
			BurningTrees = 45258,      -- Dead Trees burning
			OldShrine = 45259,         -- Mountain shrine
			CastleGarden = 45260,      -- Castle garden
			CastleEntrance = 45261,    -- Castle entrance (required)
			MarkedCount = 45262        -- Total marked spots count
		},
		BloodCrystal = {
			Quest = 45263,             -- Blood Crystal quest started
			Charged = 45264,           -- Crystal charged by A Wandering Soul
			RitualCompleted = 45265    -- 4-person ritual completed
		},
		TwineFireCount = 45266,       -- How many times fire bug used on twines
		BrokenMirrors = 45267,        -- How many mirrors broken for Lersatio access
		MarzielRitualStarted = 45268, -- Marziel ritual started (vial of blood used)
		MarzielStatuePosition = 45269, -- Player is standing on ritual tile in front of Vampire Lord Statue
		MarzielRitualCooldown = 45270 -- Cooldown to prevent multiple ritual triggers from blood decay
	},

-- Pilgrimage of Ashes Quest storage
PilgrimageOfAshes = {
	Questline = 45300,           -- Quest started by city guide
	Mission01 = 45301,           -- Spiritual Shielding blessing obtained from Norf
	Mission02 = 45302,           -- Embrace of Tibia blessing obtained from Humphrey
	Mission03 = 45303,           -- Fire of the Suns blessing obtained from Edala
	Mission04 = 45304,           -- Spark of the Phoenix blessing obtained from Kawill/Pydar
	Mission05 = 45305,           -- Wisdom of Solitude blessing obtained from Eremo
	RewardClaimed = 45306       -- 20 platinum coin reward claimed from city guide
},
KawillBlessing = 45307,         -- Spark of the Phoenix: Kawill's part obtained

	-- Annihilator storage
	AnnihilatorDone = 30081,
	AnnihilatorRewardChosen = 30082,

	postman = {
		Mission01 = 80000,
		Mission02 = 80001,
		Mission03 = 80002,
		Mission04 = 80003,
		Mission05 = 80004,
		Mission06 = 80005,
		Mission07 = 80006,
		Mission08 = 80007,
		Mission09 = 80008,
		Mission10 = 80009,
		Rank = 80010,
		MeasurementsBenjamin = 80011,
		MeasurementsChantalle = 80012,
		MeasurementsChrystal = 80013,
		MeasurementsDove = 80014,
		MeasurementsKroox = 80015,
		MeasurementsLiane = 80016,
		MeasurementsOlrik = 80017,
		Door = 80022,
		TravelCarlin = 80023,
		TravelEdron = 80024,
		TravelVenore = 80025,
		TravelCormaya = 80026
	},
	OutfitQuest = {
		Ref = 82000,
		DefaultStart = 50000,  -- XML quest start: "Outfit and Addon Quests"
		-- Assassin outfit quest
		Assassin = {
			BaseOutfit = 50080,      -- XML: 9 states
			FirstAddon = 50081,      -- XML: 8 states (Headpiece)
			SecondAddon = 18999,     -- XML: 2 states (The Red Death/Katana)
			LegacyBase = 82030       -- NPC reference
		},
		-- Citizen outfit quest
		Citizen = {
			AddonHat = 12011,        -- XML: 2 states (Feather Hat)
			AddonBackpack = 12012,   -- XML: 3 states
			AddonBackpackTimer = 12014,  -- Timer for backpack creation (changed from 12013 to avoid conflict with DruidQuest)
			AlternateHat = 50130     -- Alternative storage
		},
		-- Barbarian outfit quest
		Barbarian = {
			Quest = 12015            -- XML: 17 states (includes both addons)
		},
		-- Beggar outfit quest
		Beggar = {
			Quest = 12018,           -- XML: 6 states
			FirstAddon = 82080,      -- NPC reference
			SecondAddon = 82081      -- NPC reference
		},
		-- Druid outfit quest
		Druid = {
			Quest = 12013,           -- XML: 10 states
			BodyAddon = 82040        -- NPC reference
		},
		-- Hunter outfit quest
		Hunter = {
			Quest = 12055,           -- XML: 5 states
			HatAddon = 50135,
			AddonGlove = 50140,
			MusicSheet01 = 50141,
			MusicSheet02 = 50142,
			MusicSheet03 = 50143,
			MusicSheet04 = 50144
		},
		-- Knight outfit quest
		Knight = {
			AddonSword = 12153,      -- XML: 2 states
			AddonHelmet = 12155,     -- XML: 7 states
			AddonHelmetTimer = 82091,
			MissionHelmet = 82092,
			LegacySword = 82093,
			RamsaysHelmetDoor = 82094,
			WarriorSwordAddon = 50148
		},
		-- Mage/Summoner outfit quest
		MageSummoner = {
			QuestWand = 12061,       -- XML: 7 states
			QuestBelt = 12062,       -- XML: 2 states
			AddonHeadgear = 12064,   -- XML: 11 states
			AddonHatCloak = 82020,   -- NPC reference
			AddonBelt = 82021,       -- NPC reference
			MissionHatCloak = 82022,
			AddonWand = 82023,
			AddonWandTimer = 82024
		},
		-- Norseman outfit quest
		Norseman = {
			Quest = 12065            -- XML: 3 states (both addons)
		},
		-- Warrior outfit quest
		Warrior = {
			AddonShoulderSpike = 12067, -- XML: 7 states
			LegacyAddon = 82050      -- NPC reference
		},
		-- Wizard outfit quest
		Wizard = {
			Quest = 12066,           -- XML: 7 states (both addons)
			Addon = 82060            -- NPC reference
		},
		-- Pirate outfit quest
		Pirate = {
			SabreAddon = 50002,      -- XML: 5 states
			BaseOutfit = 82110,      -- NPC reference
			LegacySabre = 50147
		},
		-- Oriental outfit quest
		Oriental = {
			AddonHipwear = 50137,    -- XML: 2 states
			AddonHeadgear = 50138,   -- XML: 5 states
			SecondAddon = 82100      -- NPC reference
		},
		-- Shaman outfit quest
		Shaman = {
			AddonStaffMask = 15000,  -- XML: 4 states (both staff and mask)
			LegacyAddon = 82010,     -- NPC reference
			MissionStaff = 82011,
			MissionMask = 82012
		},
		-- Brotherhood/Nightmare outfit quests
		BrotherhoodOutfit = 82070,
		NightmareOutfit = 82071,
		BrotherhoodDoor = 82072,
		NightmareDoor = 82073,
		-- Golden outfit quest
		GoldenBaseOutfit = 82120,
		GoldenFirstAddon = 82121,
		GoldenSecondAddon = 50122,
		-- Nobleman outfit quest
		Nobleman = {
			AddonHat = 50145,
			AddonOutfit = 50146
		}
	},

	-- Dreamers Challenge quest storage
	DreamersChallenge = {
		Reward = 87007,
		CurrentRoom = 87000,
		FirstSeal = 87001,
		SecondSeal = 87002,
		ThirdSeal = 87003,
		FourthSeal = 87004,
		FifthSeal = 87005,
		SixthSeal = 87006,
		-- Merged from second table definition
		LeverNightmare1 = 50815,
		LeverNightmare2 = 50816,
		LeverNightmare3 = 50817,
		LeverBrotherhood1 = 50818,
		LeverBrotherhood2 = 50819,
		LeverBrotherhood3 = 50820
	},

	-- Ancient Tombs quest storage
	TheAncientTombs = {
		AshmunrahSwitches = 30014,
		DiprathSwitches = 88019,
		ThalasSwitches = 30016,
		MahrdisSwitches = 30017,
		VashresamunSwitches = 88020,
		OmrucSwitches = 88021,
		RahemosSwitches = 88022
	},

	-- Fathers Burden Quest storage
	FathersBurdenQuest = {
		Corpse = {
			Scale = 30021,
			Sinew = 30048
		},
		QuestLog = 50203,
		Status = 50205,
		Sinew = 50206,
		Wood = 50207,
		Cloth = 50208,
		Silk = 50209,
		Crystal = 50210,
		Root = 50211,
		Iron = 50212,
		Scale = 50213,
		Progress = 50214
	},

	-- Exercise Dummy storage
	Exercisedummy = {
		exaust = 98231521
	},

	-- Demon Oak storage
	DemonOak = {
		Squares = 30023,
		AxeBlowsBird = 30024,
		AxeBlowsLeft = 30025,
		AxeBlowsRight = 30026,
		AxeBlowsFace = 30029,
		Done = 30027,
		Progress = 30028
	},

	-- Tibia Tales storage
	TibiaTales = {
		DefaultStart = 81000,
		Ref = 81001,
		Questline = 81045,
		ToAppeaseTheMightyQuest = 81020,
		AritosTask = 81021,
		AgainstTheSpiderCult = 81022,
		AnInterestInBotany = 81023,
		AnInterestInBotanyChest = 81024,
		IntoTheBonePit = 81025,
		TheExterminator = 81026,
		ultimateBoozeQuest = 81027,
		NomadsLand = 81028,
		RestInHallowedGround = {
			Questline = 81002,
			HolyWater = 81003,
			Graves = {
				Grave1 = 81004,
				Grave2 = 81005,
				Grave3 = 81006,
				Grave4 = 81007,
				Grave5 = 81008,
				Grave6 = 81009,
				Grave7 = 81010,
				Grave8 = 81011,
				Grave9 = 81012,
				Grave10 = 81013,
				Grave11 = 81014,
				Grave12 = 81015,
				Grave13 = 81016,
				Grave14 = 81017,
				Grave15 = 81018,
				Grave16 = 81019
			}
		}
	},

	-- In Service of Yalahar storage
	InServiceOfYalahar = {
		WarGolemsMachine1 = 30030,
		WarGolemsMachine2 = 30031
	},

	-- Djinn War storage
	DjinnWar = {
		Faction = {
			Greeting = 50723,
			Efreet = 30033,
			Marid = 30034
		},
		Status = 88000,
		EfreetFaction = {
			Start = 88001,
			Mission01 = 88006,
			Mission02 = 88007,
			Mission03 = 88008,
			DoorToLamp = 88009,
			DoorToMaridTerritory = 88010
		},
		MaridFaction = {
			Start = 88002,
			Mission01 = 88011,
			Mission02 = 88012,
			Mission03 = 88013,
			DoorToLamp = 88014,
			DoorToEfreetTerritory = 88015,
			RataMari = 88016
		}
	},

	-- Pits of Inferno storage
	PitsOfInferno = {
		ThroneInfernatil = 30035,
		ThroneTafariel = 30036,
		ThroneVerminor = 30037,
		ThroneApocalypse = 30038,
		ThroneBazir = 30039,
		ThroneAshfalor = 30040,
		ThronePumin = 30041,
		WeaponReward = 30042,
		ShortcutHub = 30043,
		ShortcutLevers = 30044
	},

	-- Thieves Guild storage
	thievesGuild = {
		Quest = 30069,
		Mission01 = 30070,
		Mission02 = 30071,
		Mission03 = 30072,
		Mission04 = 30073,
		Mission05 = 30074,
		Mission06 = 30075,
		Mission07 = 30076,
		Mission08 = 30077,
		Reward = 30078,
		Door = 30079,
		TheatreScript = 30080
	},


	-- Wrath of the Emperor storage (player storages)
	WrathoftheEmperor = {
		Questline = 30094,
		Mission01 = 30095,
		Mission02 = 30096,
		Mission03 = 30097,
		Mission04 = 30098,
		Mission05 = 30099,
		Mission06 = 30100,
		Mission07 = 30101,
		Mission08 = 30102,
		Mission09 = 30103,
		Mission10 = 30104,
		Mission11 = 30105,
		Mission12 = 30106,
		mainReward = 30107,
		BossStatus = 30108,
		CrateStatus = 30109,
		GhostOfAPriest01 = 30110,
		GhostOfAPriest02 = 30111,
		GhostOfAPriest03 = 30112,
		GuardcaughtYou = 30113,
		InterdimensionalPotion = 30114,
		PrisonReleaseStatus = 30115,
		TeleportAccess = 30116,
		ZumtahStatus = 30117
	},

	-- Quest Chests storage
	QuestChests = {
		BananaPalm = 30118,
		BlackKnightTreeCrownArmor = 30119,
		BlackKnightTreeCrownShield = 30120,
		BlackKnightTreeKey = 30121,
		DeeperFibulaKey = 30122,
		DemonHelmetQuestDemonHelmet = 30123,
		DoubletQuest = 30124,
		FamilyBrooch = 30125,
		FirewalkerBoots = 30126,
		HoneyFlower = 30127,
		KosheiAmulet1 = 30128,
		KosheiAmulet2 = 30129,
		OldParchment = 30130,
		OutlawCampKey1 = 30131,
		OutlawCampKey2 = 30132,
		OutlawCampKey3 = 30133,
		ParchmentRoomQuest = 30134,
		SilverBrooch = 30135,
		SixRubiesQuest = 30136,
		StealFromThieves = 30137,
		WhisperMoss = 30138
	},

	-- Svargrond Arena storage
	SvargrondArena = {
		Arena = 84000,
		Greenhorn = 84001,
		Pit = 84002,
		RewardGreenhorn = 84003,
		RewardScrapper = 84004,
		RewardWarlord = 84005,
		Scrapper = 84006,
		Warlord = 84007,
		QuestLogGreenhorn = 84008,
		QuestLogScrapper = 84009,
		QuestLogWarlord = 84010,
		TrophyGreenhorn = 84011,
		TrophyScrapper = 84012,
		TrophyWarlord = 84013,
		RewardChosenGreenhorn = 84014,
		RewardChosenScrapper = 84015,
		RewardChosenWarlord = 84016
	},

	-- Hunt for the sea serpent storage
	CaptainHaba = 86003,

	-- Explorer Society storage
	ExplorerSociety = {
		bansheeDoor = 30147,
		bonelordsDoor = 30148,
		CalassaQuest = 30149,
		edronDoor = 30150,
		giantsmithhammer = 30151,
		JoiningtheExplorers = 30152,
		orcDoor = 30153,
		QuestLine = 30154,
		skullofratha = 30155,
		SpectralStone = 30156,
		TheAstralPortals = 30157,
		TheBonelordSecret = 30158,
		TheButterflyHunt = 30159,
		TheEctoplasm = 30160,
		TheElvenPoetry = 30161,
		TheIceDelivery = 30162,
		TheIceMusic = 30163,
		TheIslandofDragons = 30164,
		TheLizardUrn = 30165,
		TheMemoryStone = 30166,
		TheOrcPowder = 30167,
		ThePlantCollection = 30168,
		TheRuneWritings = 30169,
		TheSpectralDress = 30170,
		TheSpectralStone = 30171,
		urnDoor = 30172
	},

	-- The Way to Yalahar storage
	TheWayToYalahar = {
		QuestLine = 30856
	},

	-- Hidden City of Beregar storage
	hiddenCityOfBeregar = {
		BrownMushrooms = 30173,
		DefaultStart = 30174,
		DoorNorthMine = 30175,
		DoorSouthMine = 30176,
		DoorWestMine = 30177,
		GearWheel = 30178,
		GoingDown = 30179,
		JusticeForAll = 30180,
		OreWagon = 30181,
		PythiusTheRotten = 30182,
		RoyalRescue = 30183,
		SweetAsChocolateCake = 30184,
		TheGoodGuard = 30185,
		WayToBeregar = 30186,
		DoorNorthMine = 12610,
		DoorWestMine = 12611,
		DoorSouthMine = 12612
	},

	-- In Service of Yalahar storage
	InServiceofYalahar = {
		AlchemistFormula = 30194,
		BadSide = 30195,
		DoorToAzerus = 30196,
		DoorToBog = 30197,
		DoorToLastFight = 30198,
		DoorToMatrix = 30199,
		DoorToQuara = 30200,
		DoorToReward = 30201,
		GoodSide = 30202,
		MatrixReward = 30203,
		MatrixState = 30204,
		Mission01 = 30205,
		Mission02 = 30206,
		Mission03 = 30207,
		Mission04 = 30208,
		Mission05 = 30209,
		Mission06 = 30210,
		Mission07 = 30211,
		Mission08 = 30212,
		Mission09 = 30213,
		Mission10 = 30214,
		MorikSummon = 30215,
		MrWestDoor = 30216,
		MrWestStatus = 30217,
		NotesAzerus = 30218,
		NotesPalimuth = 30219,
		QuaraInky = 30220,
		QuaraSharptooth = 30221,
		QuaraSplasher = 30222,
		QuaraState = 30223,
		Questline = 30224,
		SewerPipe01 = 30225,
		SewerPipe02 = 30226,
		SewerPipe03 = 30227,
		SewerPipe04 = 30228,
		SideDecision = 30229,
		TamerinStatus = 30230,
		ResearchNotesBox = 30231,
		AzerusNotesBox = 30232,
		DiseasedBill = 30233,
		DiseasedDan = 30234,
		DiseasedFred = 30235
	},



	-- Searoutes Around Yalahar storage
	SearoutesAroundYalahar = {
		TownsCounter = 30300,
		AbDendriel = 30301,
		Darashia = 30302,
		Venore = 30303,
		Ankrahmun = 30304,
		PortHope = 30305,
		Thais = 30306,
		LibertyBay = 30307,
		Carlin = 30308
	},

	-- The Ice Islands storage
	TheIceIslands = {
		MemoryCrystal = 85000,
		Mission01 = 85001,
		Mission02 = 85002,
		Mission03 = 85003,
		Mission04 = 85004,
		Mission05 = 85005,
		Mission06 = 85006,
		Mission07 = 85007,
		Mission08 = 85008,
		Mission09 = 85009,
		Mission10 = 85010,
		Mission11 = 85011,
		Mission12 = 85012,
		Obelisk01 = 85013,
		Obelisk02 = 85014,
		Obelisk03 = 85015,
		Obelisk04 = 85016,
		PaintSeal = 85017,
		Questline = 85018,
		yakchalDoor = 85019,
		IcePassage1 = 85020,
		IcePassage2 = 85021,
		IcePassage3 = 85022
	},

	-- Children of the Revolution storage
	ChildrenoftheRevolution = {
		Questline = 30251,
		Mission00 = 30252,
		Mission01 = 30253,
		Mission02 = 30254,
		Mission03 = 30255,
		Mission04 = 30256,
		Mission05 = 30257,
		SpyBuilding01 = 30258,
		SpyBuilding02 = 30259,
		SpyBuilding03 = 30260,
		StrangeSymbols = 30261
	},

	-- Rookgaard Tutorial Island storage
	RookgaardTutorialIsland = {
		SantiagoNpcGreetStorage = 30262,
		ZirellaNpcGreetStorage = 30263,
		CarlosNpcGreetStorage = 30264,
		SantiagoQuestLog = 30265,
		ZirellaQuestLog = 30266,
		CarlosQuestLog = 30267
	},

	-- Secret Service quest storage
	secretService = {
		RottenTree = 30268,
		Quest = 12550,
		AmazonDisguiseKit = 7700,
		TBIMission01 = 12570,
		AVINMission01 = 12571,
		CGBMission01 = 12572,
		TBIMission02 = 12573,
		AVINMission02 = 12574,
		CGBMission02 = 12575,
		TBIMission03 = 12576,
		AVINMission03 = 12577,
		CGBMission03 = 12578,
		TBIMission04 = 12579,
		AVINMission04 = 12580,
		CGBMission04 = 12581,
		TBIMission05 = 12582,
		AVINMission05 = 12583,
		CGBMission05 = 12584,
		TBIMission06 = 12585,
		AVINMission06 = 12586,
		CGBMission06 = 12587,
		Mission07 = 12588
	},

	-- Unnatural Selection quest storage
	UnnaturalSelection = {
		Mission01 = 30269
	},

	-- Sam's Old Backpack storage
	SamsOldBackpack = 30270,

	-- Ghost Ship quest storage
	GhostShipQuest = 30271,

	-- Hydra Egg quest storage
	HydraEggQuest = 30272,

	-- Blood Herb quest storage
	BloodHerbQuest = 30273,

	-- The Ape City quest storage
	TheApeCity = {
		HolyApeHair = 30274,
		WitchesCapSpot = 30275,
		Started = 30284,
		Questline = 30285,
		DworcDoor = 30286,
		ParchmentDecyphering = 30287,
		Casks = 30288,
		SnakeDestroyer = 30289,
		ShamanOutfit = 30290,
		ChorDoor = 30291,
		FibulaDoor = 30292,
		CasksDoor = 30293
	},

	-- Sweety Cyclops storage
	SweetyCyclops = {
		AmuletTimer = 48,
		AmuletStatus = 49
	},

	-- Grimvale storage
	Grimvale = {
		SilverVein = 10094
	},

	-- Koshei the Deathless storage
	KosheiTheDeathless = {
		RewardDoor = 3066
	},

	-- Queen of Banshees Quest storage
	QueenOfBansheesQuest = {
		FirstSeal = 50013,
		SecondSeal = 50019,
		ThirdSeal = 50018,
		ThirdSealActive = 50025,
		FourthSeal = 50016,
		FifthSeal = 50015,
		SixthSeal = 50014,
		LastSeal = 50021,
		ThirdSealWarlocks = 50017,
		Kiss = 50020
	},

	-- Hot Cuisine Quest storage
	HotCuisineQuest = {
		QuestStart = 50022,
		CurrentDish = 50023,
		QuestLog = 50024,
		CookbookDoor = 50028
	},

	-- White Raven Monastery Quest storage
	WhiteRavenMonasteryQuest = {
		QuestLog = 50200,
		Passage = 50201,
		Diary = 50202
	},

	-- Horestis Tomb storage
	HorestisTomb = {
		JarFloor1 = 50006,
		JarFloor2 = 50007,
		JarFloor3 = 50008,
		JarFloor4 = 50009,
		JarFloor5 = 50010
	},

	-- What a Foolish Quest storage
	WhatAFoolishQuest = {
		Questline = 3900,
		Mission1 = 3901,
		Mission2 = 3902,
		Mission3 = 3903,
		Mission4 = 3904,
		Mission5 = 3905,
		Mission6 = 3906,
		Mission7 = 3907,
		Mission8 = 3908,
		Mission9 = 3909,
		Mission10 = 3910,
		Mission11 = 3911,
		PieBuying = 3912,
		PieBoxTimer = 3913,
		TriangleTowerDoor = 3914,
		EmperorBeardShave = 3915,
		JesterOutfit = 3916,
		WhoopeeCushion = 3917,
		QueenEloiseCatDoor = 3918,
		CatBasket = 3919,
		ScaredCarina = 3920,
		InflammableSulphur = 3921,
		SpecialLeaves = 3922,
		Cigar = 3923,
		Contract = 3924,
		CookieDelivery = {
			SimonTheBeggar = 3925,
			Markwin = 3926,
			Ariella = 3927,
			Hairycles = 3928,
			Djinn = 3929,
			AvarTar = 3930,
			OrcKing = 3931,
			Lorbas = 3932,
			Wyda = 3933,
			Hjaern = 3934
		},
		OldWornCloth = 3935,
		LostDisguise = 3936,
		ScaredKazzan = 3937
	},

	-- Orc King Greeting storage
	OrcKingGreeting = 3938,

	-- The Inquisition Quest storage
	TheInquisition = {
		Questline = 12160,
		Mission01 = 12161,
		Mission02 = 12162,
		Mission03 = 12163,
		Mission04 = 12164,
		Mission05 = 12165,
		Mission06 = 12166,
		Mission07 = 12167,
		WalterGuard = 12168,
		KulagGuard = 12169,
		GrofGuard = 12170,
		MilesGuard = 12171,
		TimGuard = 12172,
		StorkusVampiredust = 12173,
		Reward = 12174
	},

	-- The Shattered Isles Quest storage
	TheShatteredIsles = {
		Questline = 12175,
		Shipwrecked = 12176,
		APoemForTheMermaid = 12177,
		AccessToGoroma = 12178,
		ADjinnInLove = 12179,
		AccessToLagunaIsland = 12180,
		AccessToMeriana = 12181,
		TheCounterspell = 12182,
		EleonoreTopic = 12183,
		TheGovernorDaughter = 12184,
		TheErrand = 12185,
		WoodPiecesGiven = 12186,
		DragahsSpellbook = 12187
	},

	-- The First Dragon Quest storage
	FirstDragon = {
		Start = 4000,
		DesertTile = 4001,
		StoneSculptureTile = 4002,
		SuntowerTile = 4003,
		DragonCounter = 4004,
		TazhadurTimer = 4005,
		ChestCounter = 4006,
		KalyassaTimer = 4007,
		SecretsCounter = 4008,
		ZorvoraxTimer = 4009,
		GelidrazahAccess = 4010,
		GelidrazahTimer = 4011
	},

	-- Sea of Light Quest storage
	SeaOfLightQuest = {
		Questline = 50250,
		Mission1 = 50251,
		Mission2 = 50252,
		Mission3 = 50253,
		StudyTimer = 50254,
		LostMinesCrystal = 50255
	},

	-- Diapason storage
	Diapason = {
		Lyre = 500,
		LyreTimer = 501,
		Edala = 502,
		EdalaTimer = 503
	},

	-- Spirit Hunters Quest storage
	spiritHuntersQuest = {
		missionUm = 165163,
		tombsUse = 165164,
		charmUse = 165165,
		nightstalkerUse = 165166,
		souleaterUse = 165167,
		ghostUse = 165168
	},

	-- Deeper Banuta Shortcut storage
	DeeperBanutaShortcut = 30276,

	-- Killing in the Name of quest storage
	KillingInTheNameOf = {
		Join = 100157, -- JOIN_STOR
		Points = 2500, -- POINTSSTORAGE
		QuestStorageBase = 1500, -- QUESTSTORAGE_BASE
		KillsStorageBase = 65000, -- KILLSSTORAGE_BASE
		RepeatStorageBase = 48950, -- REPEATSTORAGE_BASE
		MissionTiquandasRevenge = 86001,
		TiquandasRevengeTeleport = 30277,
		MissionDemodras = 30279,
		DemodrasTeleport = 30278,
		GreenDjinn = 12500,
		BlueDjinn = 86000,
		Pirates = 65047,
		Minotaurs = 12700,
		Demons = 41300,
		HornedFox = 17522,
		LethalLissy = 17523,
		PromotionHuntsman = 30280, -- Promotion to Huntsman given
		PromotionRanger = 30281, -- Promotion to Ranger given
		PromotionBigGameHunter = 30282, -- Promotion to Big Game Hunter given
		PromotionTrophyHunter = 30283, -- Promotion to Trophy Hunter given
		PromotionEliteHunter = 86002 -- Promotion to Elite Hunter given
	},

	-- Rookgaard Hints storage
	RookgaardHints = 50700,

	-- Rookgaard Destiny storage
	RookgaardDestiny = 50701,

	-- Adventurers Guild storage
	AdventurersGuild = {
		Stone = 50702,
		MagicDoor = 50703,
		CharosTrav = 50724,
		FreeStone = {
			Alia = 50704,
			Amanda = 50705,
			Brewster = 50706,
			Isimov = 50707,
			Kasmir = 50708,
			Kjesse = 50709,
			Lorietta = 50710,
			Maealil = 50711,
			Quentin = 50712,
			RockWithASoftSpot = 50713,
			Tyrias = 50714,
			Yberius = 50715,
			Rahkem = 50716
		},
		GreatDragonHunt = {
			WarriorSkeleton = 50806,
			DragonCounter = 50807
		}
	},

	-- Dreamers Challenge storage


	-- Halls of Hope storage
	HallsOfHope = {
		Reward1 = 50801,
		Reward2 = 50802,
		Reward3 = 50803,
		Reward4 = 50804,
		Reward5 = 50805
	},

	-- Eruaran Greeting storage
	EruaranGreeting = 3250,

	-- Maryza Cookbook storage
	MaryzaCookbook = 50721,

	-- Combat Protection storage
	combatProtectionStorage = 50722,

	-- Block Movement storage
	blockMovementStorage = 100000,

	-- Pet Summon storage
	PetSummon = 60045,

	-- Is Training storage
	isTraining = 37,

	-- An Uneasy Alliance Quest storage
	AnUneasyAlliance = 45220,
	AnUneasyAllianceTasks = {
		MilkDelivery = 45221,
		FeedBeast = 45222,
		HonourDead = 45223,
		FoulSpirits = 45224,
		DailyTaskReset = 45225
	},

	-- Barbarian Test Quest storage
	BarbarianTest = {
		Questline = 12190,
		Mission01 = 12191,
		Mission02 = 12192,
		Mission03 = 12193,
		MeadTotalSips = 12194,
		MeadSuccessSips = 12195
	},

	-- Friends and Traders Quest storage
	FriendsAndTraders = {
		Questline = 12400,
		DefaultStart = 12404,
		TheSweatyCyclops = 12401,
		TheMermaidMarina = 12402,
		TheBlessedStake = 12403
	},




	-- The Ancient Tombs Quest storage
	AncientTombs = {
		Questline = 12100,
		Omruc = 12101,
		Thalas = 12102,
		Dipthrah = 12103,
		Mahrdis = 12104,
		Vashresamun = 12105,
		Morguthis = 12106,
		Rahemos = 12107
	},

	-- The Beginning Quest storage
	TheBeginning = {
		CockroachPlague = 50087,
		CollectingWood = 50092,
		HungryTailor = 50098
	},

	-- The New Frontier Quest storage
	TheNewFrontier = {
		Questline = 12130,
		Mission01 = 12131,
		Mission02 = 12132,
		Mission03 = 12133,
		Mission04 = 12134,
		Mission05 = 12135,
		Mission06 = 12136,
		Mission07 = 12137,
		Mission08 = 12138,
		Mission09 = 12139,
		Mission10 = 12140,
		TomeofKnowledge = 12141,
		TomeofKnowledgeLast = 12142,
		Beaver1 = 12150,
		Beaver2 = 12151,
		Beaver3 = 12143,
		BribeKing = 12144,
		BribeLeeland = 12145,
		BribeExplorerSociety = 12146,
		BribeWydrin = 12147,
		BribeTelas = 12148,
		BribeHumgolf = 12149
	},

	-- The Postman Missions Quest storage
	ThePostmanMissions = {
		Questline = 12450,
		Mission01 = 12451,
		Mission02 = 12452,
		Mission03 = 12453,
		Mission04 = 12454,
		Mission05 = 12455,
		Mission06 = 12456,
		Mission07 = 12457,
		Mission08 = 12458,
		Mission09 = 12459,
		Mission10 = 12460
	},

	-- The Thieves Guild Quest storage
	TheThievesGuild = {
		Questline = 12501,
		Mission01 = 12502,
		Mission02 = 12503,
		Mission03 = 12504,
		Mission04 = 12505,
		Mission05 = 12506,
		Mission06 = 12507,
		Mission07 = 12508,
		Mission08 = 12509,
		Mission09 = 12510,
		Mission10 = 12511
	},

	-- The Travelling Trader Quest storage
	TheTravellingTraderQuest = {
		Questline = 51201,
		Mission01 = 51202,
		Mission02 = 51203,
		Mission03 = 51204,
		Mission04 = 51205,
		Mission05 = 51206,
		Mission06 = 51207,
		Mission07 = 51208,
		Mission08 = 51209,
		Mission09 = 51210,
		Mission10 = 51211
	},



	-- Unnatural Selection Quest storage
	UnnaturalSelection = {
		Questline = 12330,
		Mission01 = 12331,
		Mission02 = 12332,
		Mission03 = 12333,
		Mission04 = 12334,
		Mission05 = 12335,
		Mission06 = 12336
	},

	-- Wrath of the Emperor Quest storage
	WrathOfTheEmperorQuest = {
		Questline = 12350,
		Mission01 = 12351,
		Mission02 = 12352,
		Mission03 = 12353,
		Mission04 = 12354,
		Mission05 = 12355,
		Mission06 = 12356,
		Mission07 = 12357,
		Mission08 = 12358,
		Mission09 = 12359,
		Mission10 = 12360
	},

	-- Spell learning storages
	Spells = {
		EternalWinter = 56001,
		HellsCore = 56002,
		RageOfTheSkies = 56003,
		WrathOfNature = 56004,
		Sharpshooter = 56005,
		SwiftFoot = 56006,
		UltimateTerraStrike = 56007,
		UltimateIceStrike = 56008,
		UltimateFlameStrike = 56009,
		UltimateEnergyStrike = 56010
	},

	-- Farmine/Kazordoon Mine Cart System
	wagonTicket = 45280,  -- Weekly mine cart ticket (stores Unix timestamp of expiration)

	-- Modern Task System (Daily/Weekly UI Tasks)
	-- Range: 90000-90999 reserved for task system
	TaskSystem = {
		Points = 90000,           -- Task points balance
		DailySeed = 90001,        -- Last date accessed (for daily reset)
		DailyCount = 90002,       -- Completions today
		WeeklySeed = 90003,       -- Last week accessed (for weekly reset)
		WeeklyCount = 90004,      -- Completions this week
		KillsStorageBase = 90100, -- 90100-90399 for kill counts (id offset: daily 100+i, weekly 200+i)
		StateStorageBase = 90400, -- 90400-90699 for task states (0=New, 1=Active, 2=Claimable, 3=Done)
		SlayerEssence = 90700,    -- Permanent +20% damage vs task monsters upgrade
		BiggerAndBadder = 90701,  -- Permanent elite monster spawn chance upgrade
		ExtendedBase = 90800,     -- 90800-90999 for extended task flags (id offset matches task id)
		-- Rank system tracking (for future implementation)
		TotalTasksCompleted = 90005,  -- Lifetime tasks completed (for rank system)
		DailyTasksCompleted = 90006,  -- Lifetime daily tasks completed
		WeeklyTasksCompleted = 90007, -- Lifetime weekly tasks completed
		CurrentRank = 90008,          -- Current rank level (for future use)
		RewardPreference = 90009,     -- 0 = EXP rewards (default), 1 = Gold rewards
		PoolVersion = 90010           -- Task pool version for migration resets
	},

	-- Dungeon System (91000-91999)
	Dungeon = {
		-- Player storages (91000-91099)
		CurrentInstance    = 91000,  -- Active instance slot ID (0 = not in dungeon)
		CurrentDungeonId   = 91001,  -- Which dungeon type they're in
		DungeonRole        = 91002,  -- 0=none, 1=tank, 2=healer, 3=dps (future)
		WeeklyLockoutBase  = 91010,  -- 91010-91029: per-dungeon weekly lockout timestamps
		DailyLockoutBase   = 91030,  -- 91030-91049: per-dungeon daily lockout timestamps
		TotalDungeonsRun   = 91050,  -- Lifetime dungeon completions
		DungeonDeaths      = 91051,  -- Deaths inside dungeons
		DungeonScore       = 91052,  -- Performance score (future ranking)
		SavedPosX          = 91060,  -- Saved position X for disconnect/reconnect
		SavedPosY          = 91061,  -- Saved position Y
		SavedPosZ          = 91062,  -- Saved position Z
	}
}

-- Check duplicates player storage keys
do
	local duplicates = {}
	for name, id in pairs(PlayerStorageKeys) do
		if duplicates[id] then error("Duplicate keyStorage: " .. id) end
		duplicates[id] = name
	end

	local __index = function(self, key)
		local keyStorage = PlayerStorageKeys[key]
		if not keyStorage then debugPrint("Invalid keyStorage: " .. key) end
		return keyStorage
	end

	setmetatable(PlayerStorageKeys, {__index = __index})
end

-- Check duplicates global storage keys
do
	local duplicates = {}
	for name, id in pairs(GlobalStorageKeys) do
		if duplicates[id] then error("Duplicate keyStorage: " .. id) end
		duplicates[id] = name
	end

	local __index = function(self, key)
		local keyStorage = GlobalStorageKeys[key]
		if not keyStorage then debugPrint("Invalid keyStorage: " .. key) end
		return keyStorage
	end

	setmetatable(GlobalStorageKeys, {__index = __index})
end

-- Alias GlobalStorage to GlobalStorageKeys for backwards compatibility
GlobalStorage = GlobalStorageKeys
