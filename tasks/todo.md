# 772 known-creature reappear (name/HP/target lost)

**Status:** complete.

**Cause:** `0x6C` erased the server known-creature slot. Re-enter was a second `0x61` while the 772 client still had the id.

**Decompile:** `SendDeleteField` keeps the slot; `SendMapObject` UPTODATE is `0x63` + id + direction.

**Fix:** keep the slot on remove; 772 `known && uptodate` writes `0x63`. 1098 unchanged (`0x61`/`0x62`).
