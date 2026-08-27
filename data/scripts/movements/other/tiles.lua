-- Switch-floor sprite pairs. The engine loads this table at startup and runs
-- step in/out + depot announce in Rust. Do not register MoveEvents here —
-- quest tiles (demon helmet, annihilator, …) stay on `:aid()` scripts.
--
-- Add later-era ids (8.6, …) as extra rows. Unused ids on a 772 map are harmless.

SteppingTiles = {
	stepIn = {
		[416] = 417,
		[426] = 425,
		[446] = 447,
		[3216] = 3217,
		[3202] = 3215,
	},
	stepOut = {
		[417] = 416,
		[425] = 426,
		[447] = 446,
		[3217] = 3216,
		[3215] = 3202,
	},
}
