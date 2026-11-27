# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitinput .
# A widget is told its settings were saved (issue #272), with Input widget of tests/fixtures/kitinput (z_kit_input_
# widget.qs says what its status line counts: g its SETTINGS events), whose package has a settings page of one
# switch. Each change the page saves calls kit_event with SETTINGS, and the frame's next pass shows it.
# App time is the script's (GRIMOIRE_SIM_CLOCK=script); the status bar's clock is held still, in UTC.
clock 1700000000
wait 3600
tap library add
tap library add.new-notebook
wait 100
tap 1564 2254
wait 300
expect screen is editor
tap panel widgets
wait 300
tap panel widgets.test.kit.inputw
wait 400
expect editor.widget.0.status is p001 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
tap panel widgets
wait 300
tap panel widgets.manage
wait 400
expect editor.packages is 3 test.kit.inputw 1.0.0 test.kit.quietw 1.0.0 test.kit.tapw 1.0.0
tap panel packages.row.0
wait 300
tap panel packages.settings
wait 400
expect editor.package.0.settings.mode is true
tap panel packages.setting.0
wait 200
expect editor.package.0.settings.mode is false
tap panel packages.setting.0
wait 200
expect editor.package.0.settings.mode is true
tap panel packages.close
wait 400
expect screen is editor
# (two changes: two SETTINGS events; the passes are the editor's own, drawing the page again under the package page)
expect editor.widget.0.status is p003 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g02 h0
quit
