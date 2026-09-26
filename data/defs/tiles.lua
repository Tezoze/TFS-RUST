-- Switch-floor sprite pairs. Loaded at startup by `stepping_tiles.rs`.
-- Add later-era ids as extra rows. Unused ids on a 772 map are harmless.

return {
	schema = 1,
	stepIn = {
		[419] = 420,
		[431] = 430,
		[452] = 453,
		[563] = 564,
		[549] = 562,
	},
	stepOut = {
		[420] = 419,
		[430] = 431,
		[453] = 452,
		[564] = 563,
		[562] = 549,
	},
}
