# settings: status_mode=1
# settings: ink_edges=smooth
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# Ink edges (2.3, #320) in Spanish and Gothic, every text checked for fit
# (z_ink_edges_8_es_fit_classic does it in Classic): Settings > Pen with the row at each setting, and the row's list,
# upright and sideways. The row starts at Smooth, named in settings.ini (Solid is the default since #329).
# rust-only
wait 3600
tap library settings
wait 100
tap library settings.cat.3
wait 100
expect settings.ink_edges is smooth
shot 01-row-smooth
tap library ink-edges
wait 100
expect library.sheet is ink-edges
shot 02-list
tap library ink-edges.1
wait 100
expect settings.ink_edges is solid
shot 03-row-solid
rotate 1
wait 400
shot 04-row-sideways
tap library ink-edges
wait 100
expect library.sheet is ink-edges
shot 05-list-sideways
tap library ink-edges.0
wait 100
expect settings.ink_edges is smooth
rotate 0
wait 400
tap library done
wait 100
quit
