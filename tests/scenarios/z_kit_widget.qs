# settings: status_mode=1
# rust-only
# env: TZ=UTC
# copy: tests/fixtures/kit .
# The package kit (issue #267) in a widget's frame, with the packages of tests/fixtures/kit (written by
# rust/tests/kitfix: `make kit-fixtures`). Kit widget draws through `grimoire_kit` alone, in both themes: a heading,
# a paragraph wrapped to the body, a row with a switch (hit region 1), and a button at the theme's size (2) with a
# chip beside it (3), each placed by the extent the one before returned. A tap saves the id of the region it hit,
# and the status line says it. Kit plain is the same module in a package whose manifest declares no capability:
# nothing it draws is shown. Kit later imports a call of a later level of the kit.
# (the status bar's clock held still, in UTC: the shots show it)
clock 1700000000
wait 3600
tap library add
tap library add.new-notebook
wait 100
tap 1564 2254
wait 300
expect screen is editor
# Gothic. The frame is 900x1100 at 776,106: its body is 872x1014 at 790,178, and only the body is drawn on.
tap panel widgets
wait 300
tap panel widgets.test.kit.widget
wait 400
expect editor.widgets is 1 test.kit.widget
expect editor.widget.0.rect is 776 106 900 1100
expect editor.widget.0.error is
expect editor.widget.0.hits is 3
expect editor.widget.0.text is Package kit|A widget drawn with Grimoire's own elements, at the theme's sizes, in the body it is given.|A row|Button|Chip
shot 01-widget-gothic
# (the body's lower part, under the elements, has nothing drawn on it)
expect display.dark.790.900.872.290 is 0
# Taps on the hit regions by their names: each reaches widget_tap with its id.
tap panel widget.0.hit.2
wait 300
expect editor.widget.0.status is 2
tap panel widget.0.hit.3
wait 300
expect editor.widget.0.status is 3
tap panel widget.0.hit.1
wait 300
expect editor.widget.0.status is 1
tap panel widget.0.close
wait 300
expect editor.widgets is 0
# Kit plain: nothing it draws is shown and it has no hit region.
tap panel widgets
wait 300
tap panel widgets.test.kit.plain
wait 400
expect editor.widgets is 1 test.kit.plain
expect editor.widget.0.error is
expect editor.widget.0.hits is 0
expect editor.widget.0.text is
expect display.dark.790.178.872.1014 is 0
shot 02-plain
tap panel widget.0.close
wait 300
# Kit later: not started; the frame says which call it needs.
tap panel widgets
wait 300
tap panel widgets.test.kit.later
wait 400
expect editor.widget.0.error is This widget needs a newer Grimoire (grimoire_kit.later_call).
shot 03-later
tap panel widget.0.close
wait 300
expect editor.widgets is 0
# Classic: the Library's Settings > Style, and the notebook again.
tap panel library
wait 500
expect screen is library
tap library settings
wait 200
tap library settings.cat.1
wait 100
tap library theme
wait 100
expect library.sheet is style
tap library theme.classic
wait 300
expect settings.theme is classic
tap library done
wait 300
tap library card.1
wait 500
expect screen is editor
tap panel widgets
wait 300
tap panel widgets.test.kit.widget
wait 400
expect editor.widgets is 1 test.kit.widget
expect editor.widget.0.error is
expect editor.widget.0.hits is 3
shot 04-widget-classic
expect display.dark.790.900.872.290 is 0
tap panel widget.0.hit.3
wait 300
expect editor.widget.0.status is 3
tap panel widget.0.hit.2
wait 300
expect editor.widget.0.status is 2
quit
