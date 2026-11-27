# settings: status_mode=1
# Ink edges (2.3, #320): Settings > Pen's last row says Smooth and opens its list of two; Back changes nothing; Solid is
# picked, saved at once, the row says it and every brush begun from then on follows it; Smooth again.
# rust-only
wait 3600
tap library settings
wait 100
expect library.sheet is settings
tap library settings.cat.3
wait 100
expect settings.ink_edges is smooth
expect ink_edges is smooth
shot 01-pen
tap library ink-edges
wait 100
expect library.sheet is ink-edges
shot 02-list
tap library ink-edges.back
wait 100
expect library.sheet is settings
expect settings.ink_edges is smooth
tap library ink-edges
wait 100
tap library ink-edges.1
wait 100
expect library.sheet is settings
expect settings.ink_edges is solid
expect ink_edges is solid
shot 03-solid
tap library ink-edges
wait 100
tap library ink-edges.0
wait 100
expect settings.ink_edges is smooth
expect ink_edges is smooth
tap library done
wait 100
expect library.sheet is none
quit
