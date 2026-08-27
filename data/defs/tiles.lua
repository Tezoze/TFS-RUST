-- Switch-floor sprite pairs. Loaded at startup by `stepping_tiles.rs`.
-- Add later-era ids as extra rows. Unused ids on a 772 map are harmless.

return {
	schema = 1,
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
