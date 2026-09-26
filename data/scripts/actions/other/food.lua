local foods = {
	[3250] = 8, -- carrot
	[3577] = 15, -- meat
	[3578] = 12, -- fish
	[3579] = 10, -- salmon
	[3580] = 17, -- northern pike
	[3581] = 4, -- shrimp
	[3582] = 30, -- ham
	[3583] = 60, -- dragon ham
	[3584] = 5, -- pear
	[3585] = 6, -- red apple
	[3586] = 13, -- orange
	[3587] = 8, -- banana
	[3588] = 1, -- blueberry
	[3589] = 18, -- coconut
	[3590] = 1, -- cherry
	[3591] = 2, -- strawberry
	[3592] = 9, -- grapes
	[3593] = 20, -- melon
	[3594] = 17, -- pumpkin
	[3595] = 8, -- carrot
	[3596] = 6, -- tomato
	[3597] = 9, -- corncob
	[3598] = 2, -- cookie
	[3599] = 2, -- candy cane
	[3600] = 10, -- bread
	[3601] = 3, -- roll
	[3602] = 8, -- brown bread
	[3606] = 6, -- egg
	[3607] = 9, -- cheese
	[3723] = 9, -- white mushroom
	[3724] = 4, -- red mushroom
	[3725] = 22, -- brown mushroom
	[3726] = 30, -- orange mushroom
	[3727] = 9, -- wood mushroom
	[3728] = 6, -- dark mushroom
	[3729] = 12, -- some mushrooms
	[3730] = 3, -- some mushrooms
	[3731] = 36, -- fire mushroom
	[3732] = 5, -- green mushroom
}

local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	local food = foods[item.itemid]
	if not food then
		return false
	end

	-- 772 `moveuse.cc:1841-1843`: `if (CurFoodTime + ObjFoodTime) > MaxFoodTime -> FEDUP`.
	-- `food_remaining` is `SKILL_FED` `Cycle` (`crskill.cc:220`), `MaxFoodTime` = 1200.
	if player:getFood() + (food * 12) > 1200 then
		player:sendTextMessage(MESSAGE_STATUS_SMALL, "You are full.")
	else
		player:feed(food * 12)
		item:remove(1)
	end
	return true
end

for id in pairs(foods) do
	action:id(id)
end

action:register()
