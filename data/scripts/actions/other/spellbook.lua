-- 772 `UseAnnouncer` case 4 → `SendEditText` / `GetSpellbook`
-- (`moveuse.cc:1947-1948`, `sending.cc:1102-1112`, `magic.cc:3830-3901`).
-- Learned / vocation instants only (`SpellKnown` on TFS domain — not ALL_SPELLS).
-- `player:getInstantSpells()` = needLearn → persist.spells; else vocation map.
-- OTB 2175 only — 2217 is fontsize-1 stored text, not a spellbook.

local function spellbookWords(words)
	local parts = {}
	for part in string.gmatch(words, "[^,]+") do
		parts[#parts + 1] = part:match("^%s*(.-)%s*$") or part
	end
	if #parts == 0 then
		return words
	end
	local glued = (parts[1] or "") .. (parts[2] or "")
	for i = 3, #parts do
		glued = glued .. " " .. parts[i]
	end
	return glued
end

local function spellbookMana(spell)
	local name = spell.name
	if name == "Summon Creature" or name == "Convince Creature" then
		return "var"
	end
	if name == "Berserk" then
		return "4*Level"
	end
	return tostring(spell.mana)
end

local spellbook = Action()

function spellbook.onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local byLevel = {}
	local maxLevel = 0
	for _, spell in ipairs(player:getInstantSpells()) do
		if spell.level > 0 then
			local level = spell.level
			local group = byLevel[level]
			if not group then
				group = {}
				byLevel[level] = group
			end
			group[#group + 1] = spell
			if level > maxLevel then
				maxLevel = level
			end
		end
	end

	local text = ""
	for level = 1, maxLevel do
		local group = byLevel[level]
		if group then
			text = text .. "Spells for Level " .. level .. "\n"
			for _, spell in ipairs(group) do
				text = text
					.. "  "
					.. spellbookWords(spell.words)
					.. " - "
					.. spell.name
					.. ": "
					.. spellbookMana(spell)
					.. "\n"
			end
			text = text .. "\n"
		end
	end

	player:showTextDialog(item:getId(), text)
	return true
end

spellbook:id(2175)
spellbook:register()
