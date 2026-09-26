local transformItems = {
	[2108] = 2109, [2109] = 2108, -- street lamp
	[2334] = 2335, -- table
	[2336] = 2337, [2337] = 2336, -- table
	[2338] = 2339, [2339] = 2338, -- table
	[2340] = 2341, [2341] = 2340, -- table
	[2535] = 2536, [2536] = 2535, -- oven
	[2537] = 2538, [2538] = 2537, -- oven
	[2539] = 2540, [2540] = 2539, -- oven
	[2541] = 2542, [2542] = 2541, -- oven
	[2772] = 2773, [2773] = 2772, -- lever
	[2907] = 2908, [2908] = 2907, -- wall lamp
	[2909] = 2910, [2910] = 2909, -- wall lamp
	[2928] = 2929, [2929] = 2928, -- torch bearer
	[2930] = 2931, [2931] = 2930, -- torch bearer
	[2934] = 2935, [2935] = 2934, -- table lamp
	[2936] = 2937, [2937] = 2936, -- wall lamp
	[2938] = 2939, [2939] = 2938, -- wall lamp
	[2062] = 2063, [2063] = 2062, -- sacred statue
	[2064] = 2065, [2065] = 2064, -- sacred statue
	[2116] = 2117, [2117] = 2116, -- bamboo lamp
	[2940] = 2941, [2941] = 2940, -- torch bearer
	[2942] = 2943, [2943] = 2942, -- torch bearer
	[2944] = 2945, [2945] = 2944, -- wall lamp
	[2946] = 2947, [2947] = 2946, -- wall lamp
}

local transformTo = Action()

function transformTo.onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local transformIds = transformItems[item:getId()]
	if not transformIds then
		return false
	end

	item:transform(transformIds)
	return true
end

for i, v in pairs(transformItems) do
	transformTo:id(i)
end

transformTo:register()
