Place sprite atlas files here.

Loaded at runtime:
- `images/atlas.png`
- `images/atlas.meta`

`atlas.meta` format:
- `sheet <path>`: sprite sheet image path (for example `images/atlas.png`)
- `tile <w> <h>`: frame tile size
- `clip <name> fps=<n> loop=<0|1>`: start clip
- `frame <tx> <ty>`: add frame in tile coordinates
- `endclip`: optional clip terminator

Example:
```
sheet images/atlas.png
tile 16 16

clip player_idle_down fps=6 loop=1
frame 0 0
frame 1 0

clip player_walk_down fps=10 loop=1
frame 2 0
frame 3 0
frame 4 0
frame 5 0
```

Expected clip names used by current renderer:
- Player: `player_idle_up/down/left/right`, `player_walk_up/down/left/right`
- NPC: `npc_idle_down` (fallback `npc_idle`)
- Enemy: `enemy_idle`

If atlas or clips are missing, rendering falls back to colored rectangles.
