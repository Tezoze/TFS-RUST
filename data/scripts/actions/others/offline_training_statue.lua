-- Offline Training Statue (Item ID 18488)
-- Works like beds - shows a skill selection dialog, then activates offline training and logs player out

local MODAL_WINDOW_ID = 9999 -- Unique ID for this modal window (avoid conflicts)

local offlineTrainingStatue = Action()

function offlineTrainingStatue.onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Check premium
	if not player:isPremium() then
		player:sendCancelMessage(RETURNVALUE_YOUNEEDPREMIUMACCOUNT)
		return true
	end

	-- Check PZ lock
	if player:isPzLocked() then
		player:sendCancelMessage("You cannot use this statue while in a fight.")
		return false
	end

	-- Create the modal window (same as bed dialog)
	local window = ModalWindow(MODAL_WINDOW_ID, "Choose a Skill", "Please choose a skill:")
	
	-- Add skill choices based on vocation
	local vocation = player:getVocation()
	local vocationId = vocation:getId()
	
	-- Knights (vocation 4, 8): melee, distance
	if vocationId == 4 or vocationId == 8 then
		window:addChoice(SKILL_SWORD, "Melee Fighting and Shielding")
		window:addChoice(SKILL_DISTANCE, "Distance Fighting and Shielding")
	-- Paladins (vocation 3, 7): distance only
	elseif vocationId == 3 or vocationId == 7 then
		window:addChoice(SKILL_DISTANCE, "Distance Fighting and Shielding")
	-- Mages (vocation 1, 2, 5, 6): magic level only
	elseif vocationId == 1 or vocationId == 2 or vocationId == 5 or vocationId == 6 then
		window:addChoice(SKILL_MAGLEVEL, "Magic Level and Shielding")
	else
		-- No vocation or unknown - show nothing useful
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You cannot use offline training.")
		return true
	end
	
	-- Add buttons
	window:addButton(1, "Okay")
	window:addButton(0, "Cancel")
	
	window:setDefaultEnterButton(1)
	window:setDefaultEscapeButton(0)
	window:setPriority(true)
	
	-- Send to player
	window:sendToPlayer(player)
	
	return true
end

offlineTrainingStatue:id(18488)
offlineTrainingStatue:register()

-- Creature event to handle the modal window response
local modalHandler = CreatureEvent("OfflineTrainingStatueModal")

function modalHandler.onModalWindow(player, modalWindowId, buttonId, choiceId)
	if modalWindowId ~= MODAL_WINDOW_ID then
		return true
	end
	
	-- Check if player cancelled
	if buttonId == 0 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Offline training cancelled.")
		return true
	end
	
	-- Validate the selected skill (basic sanity check)
	local skill = choiceId
	if skill ~= SKILL_SWORD and skill ~= SKILL_DISTANCE and skill ~= SKILL_MAGLEVEL then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Invalid skill selected.")
		return true
	end
	
	-- Set offline training skill and log out
	player:setOfflineTrainingSkill(skill)
	player:remove()
	
	return true
end

modalHandler:register()

-- Login event to register the modal window handler
local loginEvent = CreatureEvent("OfflineTrainingStatueLogin")

function loginEvent.onLogin(player)
	player:registerEvent("OfflineTrainingStatueModal")
	return true
end

loginEvent:register()
