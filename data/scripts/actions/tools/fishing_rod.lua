local waterItems = {
	[4597] = {canFish = true, transformId = 4609},
	[4598] = {canFish = true, transformId = 4610},
	[4599] = {canFish = true, transformId = 4611},
	[4600] = {canFish = true, transformId = 4612},
	[4601] = {canFish = true, transformId = 4613},
	[4602] = {canFish = true, transformId = 4614},
	[618] = {canFish = true, transformId = 620},
	[4809] = {canFish = true, transformId = 4609},
	[4810] = {canFish = true, transformId = 4610},
	[4811] = {canFish = true, transformId = 4611},
	[4812] = {canFish = true, transformId = 4612},
	[4813] = {canFish = true, transformId = 4613},
	[4814] = {canFish = true, transformId = 4614},
	[619] = {canFish = false},
	[620] = {canFish = false},
	[622] = {canFish = false},
	[4603] = {canFish = false},
	[4604] = {canFish = false},
	[4605] = {canFish = false},
	[4606] = {canFish = false},
	[4607] = {canFish = false},
	[4608] = {canFish = false},
	[4609] = {canFish = false},
	[4610] = {canFish = false},
	[4611] = {canFish = false},
	[4612] = {canFish = false},
	[4613] = {canFish = false},
	[4614] = {canFish = false},
	[4614] = {canFish = false},
	[4653] = {canFish = false},
	[4654] = {canFish = false},
	[4655] = {canFish = false},
	[747] = {canFish = false},
	[748] = {canFish = false},
	[749] = {canFish = false},
	[750] = {canFish = false},
	[751] = {canFish = false},
	[752] = {canFish = false},
	[753] = {canFish = false},
	[754] = {canFish = false},
	[755] = {canFish = false},
	[756] = {canFish = false},
	[757] = {canFish = false},
	[758] = {canFish = false},
}

local useWorms = true

local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	local targetId = target.itemid
	local water = waterItems[targetId]
	
	if not water then
		return false
	end

	toPosition:sendMagicEffect(CONST_ME_LOSEENERGY)
	
	if water.canFish then
		player:addSkillTries(SKILL_FISHING, 1)
	end
	
	if water.canFish and formulas.fishingSuccess(player:getEffectiveSkillLevel(SKILL_FISHING)) then
		if useWorms and not player:removeItem(3492, 1) then
			return true
		end
			
		local parent = item:getParent()
		if not parent:addItem(3578, 1) then
			Tile(item:getPosition()):addItem(3578, 1)
		end

		target:transform(water.transformId)
		target:decay()
	end
	return true
end

action:id(3483)
action:allowFarUse(true)
action:register()