-- Native `TPlayer::Death` already sends "You are dead.\n" (`crplayer.cc:333`).
-- Keep the event registered so `login.lua` `registerEvent("PlayerDeath")` succeeds.
local creatureevent = CreatureEvent("PlayerDeath")

function creatureevent.onDeath(_player)
end

creatureevent:register()
