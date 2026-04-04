local fireFieldDebug = MoveEvent()
function fireFieldDebug.onStepIn(creature, item, position, fromPosition)
    if item:getId() == 1487 or item:getId() == 1488 then
        print("Player " .. creature:getName() .. " stepped on fire field ID " .. item:getId())
        if creature:getCondition(CONDITION_FIRE) then
            print("Fire condition already active")
        else
            print("Applying fire condition")
        end
    end
    return true
end
fireFieldDebug:type("stepin")
fireFieldDebug:register()