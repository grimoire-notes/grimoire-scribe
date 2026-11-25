# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kit .
# The package kit (issue #267) in an app, with the packages of tests/fixtures/kit (written by rust/tests/kitfix:
# `make kit-fixtures`). Kit app draws through `grimoire_kit` alone, in both themes: from the theme's margin a page
# title, a wrapped paragraph and, each placed by the extent the one before returned, a row with a switch, a button
# with a chip beside it, a field, a select and a checkbox; an icon button; a pager; and, given no height, the row
# Settings > System draws fourth, at the place Settings draws it. Its elements given an id are its hit regions,
# which the runner taps by name (library.app.hit.N); each touch adds one to the count its status line says, as each
# drawing pass does. Kit later app imports a call of a later level of the kit.
# (the status bar's clock held still, in UTC: the shots show it)
clock 1700000000
wait 3600
expect library.apps is test.kit.app test.kit.lateapp
# Gothic. The app opens and draws once; it has ten hit regions, and each element's text in the probe.
tap library apps
wait 300
tap library app.test.kit.app
wait 400
expect library.app.open is 1
expect library.app.error is
expect library.app.status is 000001
expect library.app.hits is 10
expect library.app.text is Package kit|An app drawn with Grimoire's own elements, at the theme's sizes: a row, a button, a chip, a field, a select and a checkbox.|A row|Button|Chip||Search|Name|A checkbox|Darker lines in imported PDFs: Off
shot 01-app-gothic
# (the row at Settings' place: the same dark pixels as Settings' own row below)
expect display.dark.656.716.1154.118 is 18346
# A tap on hit region 2 (the button) by its name: the touch's two phases and the pass after each.
tap library app.hit.2
wait 300
expect library.app.status is 000005
tap library app.hit.10
wait 300
expect library.app.status is 000009
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
# Settings > System: its fourth row, where the app drew its kit row given no height.
tap library settings
wait 400
expect library.sheet is settings
tap library settings.cat.5
wait 300
expect display.dark.656.716.1154.118 is 18346
shot 02-settings-system-gothic
# Classic: Settings > Style, and System again.
tap library settings.cat.1
wait 100
tap library theme
wait 300
expect settings.theme is classic
tap library settings.cat.5
wait 300
expect display.dark.656.716.1154.118 is 12393
shot 03-settings-system-classic
tap library done
wait 300
expect library.sheet is none
tap library apps
wait 300
tap library app.test.kit.app
wait 400
expect library.app.open is 1
expect library.app.error is
expect library.app.hits is 10
expect display.dark.656.716.1154.118 is 12393
shot 04-app-classic
tap library app.hit.7
wait 300
expect library.app.status is 000005
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
# Kit later app imports grimoire_kit.later_call, which this Grimoire does not have: it is not started, and the
# toast names the call.
tap library apps
wait 300
tap library app.test.kit.lateapp
wait 400
expect library.app.open is 0
expect toast is This app needs a newer Grimoire (grimoire_kit.later_call).
shot 05-later
quit
