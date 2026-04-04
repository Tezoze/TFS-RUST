-- Henricus (NpcBuilder — The Inquisition Quest)
local npc = NpcBuilder:new("Henricus",
	{lookType = 132, lookHead = 79, lookLegs = 96})
npc:greetMessage("Greetings, fellow {believer} |PLAYERNAME|!")
npc:farewellMessage("Always be on guard, |PLAYERNAME|!")
npc:walkawayMessage("This ungraceful haste is most suspicious!")

local flaskCost = 1000

-- Simple keyword responses
npc:addKeyword("paladin", "It's a shame that only a few paladins still use their abilities to further the cause of the gods of good. Too many paladins have become selfish and greedy.")
npc:addKeyword("knight", "Nowadays, most knights seem to have forgotten the noble cause to which all knights were bound in the past. Only a few have remained pious, serve the gods and follow their teachings.")
npc:addKeyword("sorcerer", "Those who wield great power have to resist great temptations. We have the burden to eliminate all those who give in to the temptations.")
npc:addKeyword("druid", "The druids here still follow the old rules. Sadly, the druids of Carlin have left the right path in the last years.")
npc:addKeyword("dwarf", "The dwarfs are allied with Thais but follow their own obscure religion. Although dwarfs keep mostly to themselves, we have to observe this alliance closely.")
npc:addKeyword("kazordoon", "The dwarfs are allied with Thais but follow their own obscure religion. Although dwarfs keep mostly to themselves, we have to observe this alliance closely.")
npc:addKeyword("elves", "Those elves are hardly any more civilised than orcs. They can become a threat to mankind at any time.")
npc:addKeyword("ab'dendriel", "Those elves are hardly any more civilised than orcs. They can become a threat to mankind at any time.")
npc:addKeyword("venore", "Venore is somewhat difficult to handle. The merchants have a close eye on our activities in their city and our authority is limited there. However, we will use all of our influence to prevent a second Carlin.")
npc:addKeyword("drefia", "Drefia used to be a city of sin and heresy, just like Carlin nowadays. One day, the gods decided to destroy this town and to erase all evil there.")
npc:addKeyword("darashia", "Darashia is a godless town full of mislead fools. One day, it will surely share the fate of its sister town Drefia.")
npc:addKeyword("demon", "Demons exist in many different shapes and levels of power. In general, they are servants of the dark gods and command great powers of destruction.")
npc:addKeyword("carlin", "Carlin is a city of sin and heresy. After the reunion of Carlin with the kingdom, the inquisition will have much work to purify the city and its inhabitants.")
npc:addKeyword("zathroth", "We can see his evil influence almost everywhere. Keep your eyes open or the dark one will lead you on the wrong way and destroy you.")
npc:addKeyword("crunor", "The church of Crunor works closely together with the druid guild. This makes a cooperation sometimes difficult.")
npc:addKeyword("gods", "We owe to the gods of good our creation and continuing existence. If it weren't for them, we would surely fall prey to the minions of the vile and dark gods.")
npc:addKeyword("church", "The churches of the gods united to fight heresy and dark magic. They are the shield of the true believers, while the inquisition is the sword that fights all enemies of virtuousness.")
npc:addKeyword("inquisitor", "The churches of the gods entrusted me with the enormous and responsible task to lead the inquisition. I leave the field work to inquisitors who I recruit from fitting people that cross my way.")
npc:addKeyword("believer", "Belive on the gods and they will show you the path.")
npc:addKeyword("job", "By edict of the churches I'm the Lord Inquisitor.")
npc:addKeyword("name", "I'm Henricus, the Lord Inquisitor.")

-- Multi-message keyword responses
npc:addKeyword("dark", {
	"The dark powers are always present. If a human shows only the slightest weakness, they try to corrupt him and to lure him into their service. ...",
	"We must be constantly aware of evil that comes in many disguises."
})
npc:addKeyword("king", {
	"The Thaian kings are crowned by a representative of the churches. This means they reign in the name of the gods of good and are part of the godly plan for humanity. ...",
	"As nominal head of the church of Banor, the kings aren't only worldly but also spiritual authorities. ...",
	"The kings fund the inquisition and sometimes provide manpower in matters of utmost importance. The inquisition, in return, protects the realm from heretics and individuals that aim to undermine the holy reign of the kings."
})
npc:addKeyword("banor", {
	"In the past, the order of Banor was the only order of knighthood in existence. In the course of time, the order concentrated more and more on spiritual matters rather than on worldly ones. ...",
	"Nowadays, the order of Banor sanctions new orders and offers spiritual guidance to the fighters of good."
})
npc:addKeyword("fardos", "The priests of Fardos are often mystics who have secluded themselves from worldly matters. Others provide guidance and healing to people in need in the temples.")
npc:addKeyword("uman", {
	"The church of Uman oversees the education of the masses as well as the doings of the sorcerer and druid guilds. It decides which lines of research are in accordance with the will of Uman and which are not. ...",
	"Concerned, the inquisition watches the attempts of these guilds to become more and more independent and to make own decisions. ...",
	"Unfortunately, the sorcerer guild has become dangerously influential and so the hands of our priests are tied due to political matters ...",
	"The druids lately claim that they are serving Crunor's will and not Uman's. Such heresy could only become possible with the independence of Carlin from the Thaian kingdom. ...",
	"The spiritual centre of the druids switched to Carlin where they have much influence and cannot be supervised by the inquisition."
})
npc:addKeyword("fafnar", {
	"Fafnar is mostly worshipped by the peasants and farmers in rural areas. ...",
	"The inquisition has a close eye on these activities. Simply people tend to mix local superstitions with the teachings of the gods. This again may lead to heretical subcults."
})
npc:addKeyword("edron", {
	"Edron illustrates perfectly why the inquisition is needed and why we need more funds and manpower. ...",
	"Our agents were on their way to investigate certain occurrences there when some faithless knights fled to some unholy ruins. ...",
	"We were unable to wipe them out and the local order of knighthood was of little help. ...",
	"It's almost sure that something dangerous is going on there, so we have to continue our efforts."
})
npc:addKeyword("ankrahmun", {
	"Even though they claim differently, this city is in the firm grip of Zathroth and his evil minions. Their whole twisted religion is a mockery of the teachings of our gods ...",
	"As soon as we have gathered the strength, we should crush this city once and for all."
})

-- The Inquisition Quest logic
npc:onSay(function(npc, player, message, builder)
	local npcId = npc:getId()
	local cid = player:getId()
	local s = InstanceState.get(npcId, cid)
	if not s then return false end
	local lower = message:lower()
	local questline = player:getStorageValue(Storage.TheInquisition.Questline) or 0
	local totalBlessPrice = math.floor(StdModule.calculateRegularBlessingCost(player:getLevel()) * 5 * 1.1)

	-- Join the inquisition
	if lower:find("join") then
		if questline < 1 then
			builder:say(npc, "Do you want to join the inquisition?", player)
			s.topic = 2
			return true
		end

	-- Blessing (quest complete only)
	elseif lower:find("blessing") or lower:find("bless") then
		if questline == 25 then
			builder:say(npc, "Do you want to receive the blessing of the inquisition - which means all five available blessings - for " .. totalBlessPrice .. " gold?", player)
			s.topic = 7
		else
			builder:say(npc, "You cannot get this blessing unless you have completed The Inquisition Quest.", player)
			s.topic = 0
		end
		return true

	-- Flask purchase
	elseif lower:find("flask") or lower:find("special flask") then
		if questline >= 12 then
			builder:say(npc, "Do you want to buy the special flask of holy water for " .. flaskCost .. " gold?", player)
			s.topic = 8
		else
			builder:say(npc, "You do not need this flask right now.", player)
			s.topic = 0
		end
		return true

	-- Mission / Report
	elseif lower:find("mission") or lower:find("report") then
		if questline < 1 then
			builder:say(npc, "Do you want to join the inquisition?", player)
			s.topic = 2
		elseif questline == 1 then
			builder:sayMultiple(npc, {
				"Let's see if you are worthy. Take an inquisitor's field guide from the box in the back room. ...",
				"Follow the instructions in the guide to talk to the Thaian guards that protect the walls and gates of the city and test their loyalty. Then report to me about your {mission}."
			}, player)
			player:setStorageValue(Storage.TheInquisition.Questline, 2)
			player:setStorageValue(Storage.TheInquisition.Mission01, 1)
			s.topic = 0
		elseif questline == 2 then
			builder:say(npc, "Your current mission is to investigate the reliability of certain guards. Are you done with that mission?", player)
			s.topic = 3
		elseif questline == 3 then
			builder:sayMultiple(npc, {
				"Listen, we have information about a heretic coven that hides in a mountain called the Big Old One. The witches reach this cursed place on flying brooms and think they are safe there. ...",
				"I've arranged a flying carpet that will bring you to their hideout. Travel to Femor Hills and tell the carpet pilot the codeword 'eclipse' ...",
				"He'll bring you to your destination. At their meeting place, you'll find a cauldron in which they cook some forbidden brew ...",
				"Use this vial of holy water to destroy the brew. Also steal their grimoire and bring it to me."
			}, player)
			player:setStorageValue(Storage.TheInquisition.Questline, 4)
			player:setStorageValue(Storage.TheInquisition.Mission02, 1)
			player:addItem(7494, 1)
			s.topic = 0
		elseif questline == 5 then
			if player:removeItem(8702, 1) then
				builder:sayMultiple(npc, {
					"I think it's time to truly test your abilities. One of our allies has requested assistance. I think you are just the right person to help him ...",
					"Storkus is an old and grumpy dwarf who works as a vampire hunter since many, many decades. He's quite successful but even he has his limits. ...",
					"So occasionally, we send him help. In return he trains and tests our recruits. It's an advantageous agreement for both sides ...",
					"You'll find him in his cave at the mountain outside of Kazordoon. He'll tell you about your next mission."
				}, player)
				player:setStorageValue(Storage.TheInquisition.Questline, 6)
				player:setStorageValue(Storage.TheInquisition.Mission02, 3)
				player:setStorageValue(Storage.TheInquisition.Mission03, 1)
			else
				builder:say(npc, "You need bring me the witches' grimoire.", player)
			end
			s.topic = 0
		elseif questline > 5 and questline < 11 then
			builder:say(npc, "Your current mission is to help the vampire hunter Storkus. Are you done with that mission?", player)
			s.topic = 4
		elseif questline == 11 then
			builder:sayMultiple(npc, {
				"We've got a report about an abandoned and haunted house in Liberty Bay. I want you to examine this house. It's the only ruin in Liberty Bay so you should have no trouble finding it. ...",
				"There's an evil being somewhere. I assume that it will be easier to find the right spot at night. Use this vial of holy water on that spot to drive out the evil being."
			}, player)
			player:setStorageValue(Storage.TheInquisition.Questline, 12)
			player:setStorageValue(Storage.TheInquisition.Mission04, 1)
			player:addItem(7494, 1)
			s.topic = 0
		elseif questline == 12 or questline == 13 then
			builder:say(npc, "Your current mission is to exorcise an evil being from a house in Liberty Bay. Are you done with that mission?", player)
			s.topic = 5
		elseif questline == 14 then
			builder:sayMultiple(npc, {
				"You've handled heretics, witches, vampires and ghosts. Now be prepared to face the most evil creatures we are fighting - demons. Your new task is extremely simple, though far from easy. ...",
				"Go and slay demonic creatures wherever you find them. Bring me 20 of their essences as a proof of your accomplishments."
			}, player)
			player:setStorageValue(Storage.TheInquisition.Questline, 15)
			player:setStorageValue(Storage.TheInquisition.Mission05, 1)
			s.topic = 0
		elseif questline == 15 then
			if player:removeItem(6500, 20) then
				builder:sayMultiple(npc, {
					"You're indeed a dedicated protector of the true believers. Don't stop now. Kill as many of these creatures as you can. ...",
					"I also have a reward for your great efforts. Talk to me about your {demon hunter outfit} anytime from now on. Afterwards, let's talk about the next mission that's awaiting you."
				}, player)
				player:setStorageValue(Storage.TheInquisition.Questline, 16)
				player:setStorageValue(Storage.TheInquisition.Mission05, 2)
			else
				builder:say(npc, "You need 20 of them.", player)
			end
			s.topic = 0
		elseif questline == 17 then
			builder:sayMultiple(npc, {
				"We've got information about something very dangerous going on on the isle of Edron. The demons are preparing something there ...",
				"Something that is a threat to all of us. Our investigators were able to acquire vital information before some of them were slain by a demon named Ungreez. ...",
				"It'll be your task to take revenge and to kill that demon. You'll find him in the depths of Edron. Good luck."
			}, player)
			player:setStorageValue(Storage.TheInquisition.Questline, 18)
			player:setStorageValue(Storage.TheInquisition.Mission06, 1)
			player:registerEvent("InquisitionUngreez")
			s.topic = 0
		elseif questline == 19 then
			builder:sayMultiple(npc, {
				"So the beast is finally dead! Thank the gods. At least some things work out in our favour ...",
				"Our other operatives were not that lucky, though. But you will learn more about that in your next {mission}."
			}, player)
			player:setStorageValue(Storage.TheInquisition.Questline, 20)
			player:setStorageValue(Storage.TheInquisition.Mission06, 3)
			player:unregisterEvent("InquisitionUngreez")
			player:addOutfitAddon(288, 1)
			player:addOutfitAddon(289, 1)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
			s.topic = 0
		elseif questline == 20 then
			builder:say(npc, "Destroy the shadow nexus using this vial of holy water and kill all demon lords.", player)
			player:setStorageValue(Storage.TheInquisition.Questline, 21)
			player:setStorageValue(Storage.TheInquisition.Mission07, 1)
			player:registerEvent("InquisitionBosses")
			player:addItem(7494, 1)
			s.topic = 0
		elseif questline == 21 or questline == 22 then
			builder:say(npc, "Your current mission is to destroy the shadow nexus in the Demon Forge. Are you done with that mission?", player)
			s.topic = 6
		end
		return true

	-- Outfit
	elseif lower:find("outfit") then
		if questline == 16 then
			builder:say(npc, "Here is your demon hunter outfit. You deserve it. Unlock more addons by completing more missions.", player)
			player:setStorageValue(Storage.TheInquisition.Questline, 17)
			player:setStorageValue(Storage.TheInquisition.Mission05, 3)
			player:addOutfit(288, 0)
			player:addOutfit(289, 0)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
			s.topic = 0
			return true
		elseif questline == 23 then
			builder:say(npc, "Here is the final addon for your demon hunter outfit. Congratulations!", player)
			player:setStorageValue(Storage.TheInquisition.Questline, 24)
			player:setStorageValue(Storage.TheInquisition.Mission07, 4)
			player:addOutfitAddon(288, 1)
			player:addOutfitAddon(289, 1)
			player:addOutfitAddon(288, 2)
			player:addOutfitAddon(289, 2)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
			player:addAchievement('Demonbane')
			s.topic = 0
			return true
		end

	-- Yes
	elseif lower == "yes" then
		if s.topic == 2 then
			builder:say(npc, "So be it. Now you are a member of the inquisition. You might ask me for a {mission} to raise in my esteem.", player)
			player:setStorageValue(Storage.TheInquisition.Questline, 1)
			s.topic = 0
			return true
		elseif s.topic == 3 then
			if player:getStorageValue(Storage.TheInquisition.WalterGuard) == 1
				and player:getStorageValue(Storage.TheInquisition.KulagGuard) == 1
				and player:getStorageValue(Storage.TheInquisition.GrofGuard) == 1
				and player:getStorageValue(Storage.TheInquisition.MilesGuard) == 1
				and player:getStorageValue(Storage.TheInquisition.TimGuard) == 1 then
				builder:sayMultiple(npc, {
					"Indeed, this is exactly what my other sources told me. Of course I knew the outcome of this investigation in advance. This was just a test. ...",
					"Well, now that you've proven yourself as useful, you can ask me for another mission. Let's see if you can handle some field duty, too."
				}, player)
				player:setStorageValue(Storage.TheInquisition.Questline, 3)
				player:setStorageValue(Storage.TheInquisition.Mission01, 7)
			else
				builder:say(npc, "You haven't done your mission yet.", player)
			end
			s.topic = 0
			return true
		elseif s.topic == 4 then
			if questline == 10 then
				builder:say(npc, "Good, you've returned. Your skill in practical matters seems to be useful. If you're ready for a further mission, just ask.", player)
				player:setStorageValue(Storage.TheInquisition.Questline, 11)
				player:setStorageValue(Storage.TheInquisition.Mission03, 6)
			else
				builder:say(npc, "You haven't done your mission with {Storkus} yet.", player)
			end
			s.topic = 0
			return true
		elseif s.topic == 5 then
			if questline == 13 then
				builder:say(npc, "Well, this was an easy task, but your next mission will be much more challenging.", player)
				player:setStorageValue(Storage.TheInquisition.Questline, 14)
				player:setStorageValue(Storage.TheInquisition.Mission04, 3)
			else
				builder:say(npc, "You haven't done your mission yet.", player)
			end
			s.topic = 0
			return true
		elseif s.topic == 6 then
			if questline == 22 then
				builder:sayMultiple(npc, {
					"Incredible! You're a true defender of faith! I grant you the title of a High Inquisitor for your noble deeds. From now on you can obtain the blessing of the inquisition which makes the pilgrimage of ashes obsolete ...",
					"The blessing of the inquisition will bestow upon you all available blessings for the price of 60000 gold. Also, don't forget to ask me about your {outfit} to receive the final addon as demon hunter."
				}, player)
				player:setStorageValue(Storage.TheInquisition.Questline, 23)
				player:setStorageValue(Storage.TheInquisition.Mission07, 3)
				player:unregisterEvent("InquisitionBosses")
				player:addAchievement('High Inquisitor')
			else
				builder:say(npc, "Come back when you have destroyed the shadow nexus.", player)
			end
			s.topic = 0
			return true
		elseif s.topic == 7 then
			local blessingCount = 0
			for i = 1, 5 do
				if player:hasBlessing(i) then
					blessingCount = blessingCount + 1
				end
			end
			if blessingCount == 5 then
				builder:say(npc, "You already have been blessed!", player)
			elseif player:removeMoneyNpc(totalBlessPrice) then
				builder:say(npc, "You have been blessed by all of five gods!, |PLAYERNAME|.", player)
				for b = 1, 5 do
					player:addBlessing(b)
				end
				player:getPosition():sendMagicEffect(CONST_ME_HOLYAREA)
			else
				builder:say(npc, "Come back when you have enough money.", player)
			end
			s.topic = 0
			return true
		elseif s.topic == 8 then
			if player:removeMoneyNpc(flaskCost) then
				builder:say(npc, "Here is your new flask!, |PLAYERNAME|.", player)
				player:addItem(7494, 1)
			else
				builder:say(npc, "Come back when you have enough money.", player)
			end
			s.topic = 0
			return true
		end

	-- No
	elseif lower == "no" then
		if s.topic > 0 then
			builder:say(npc, "Then no.", player)
			s.topic = 0
			return true
		end
	end

	return false
end)

npc:register()
