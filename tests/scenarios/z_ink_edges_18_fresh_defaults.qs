# rust-only
# The defaults of #329 on a fresh data root (no settings.ini): Ink edges is Solid and the status bar is small. The
# brushes follow Solid from the start (app_init sets it from the settings before the first screen is drawn); Settings
# > Pen's Ink edges row says Solid, and its list keeps it; the Status bar page has the small size, and so has the bar
# over a notebook's page. (That the first save writes no ink_edges line and status_size=0 is test_store's
# fresh_settings_are_solid_and_small: the runner has no probe of the file.)
wait 3600
expect settings.ink_edges is solid
expect ink_edges is solid
expect settings.status_size is 0
expect settings.status_in_notebooks is 1
tap library settings
wait 100
expect library.sheet is settings
tap library settings.cat.3
wait 100
expect library.ui_has.ink-edges is 1
expect settings.ink_edges is solid
shot 01-pen-solid
tap library ink-edges
wait 100
expect library.sheet is ink-edges
shot 02-list
tap library ink-edges.back
wait 100
expect library.sheet is settings
expect settings.ink_edges is solid
expect ink_edges is solid
tap library settings.cat.2
wait 100
tap library status-bar
wait 300
expect library.ui_has.display.size is 1
expect settings.status_size is 0
shot 03-status-bar-small
tap library display.back
wait 300
tap library done
wait 100
expect library.sheet is none
tap library add
tap library add.new-notebook
wait 100
tap library create
wait 300
expect screen is editor
expect status.visible is 1
expect status.zone is 0 0 1860 72
expect ink_edges is solid
shot 04-notebook-small-bar
quit
