# settings: status_mode=1
# rust-only
# copy: tests/fixtures/widgethost .
# What the widget host lets an installed module do (issue #208), with the packages of tests/fixtures/widgethost
# (written by rust/tests/widgetfix: `make widget-fixtures`). Each is a module written for one rule of the host,
# which it reaches and passes.
wait 3600
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
# Wide draws a black button from far above and left of its body to far below and right of it, under a hit region
# as large. The frame is 400x300 at 1276,28: its body is 372x214 at 1290,100, and only the body is drawn on.
tap panel widgets
wait 300
expect editor.widgets.open is 1
tap panel widgets.test.host.wide
wait 400
expect editor.widgets is 1 test.host.wide
expect editor.widget.0.rect is 1276 28 400 300
expect editor.widget.0.error is
shot 01-wide
# (every pixel of the body is black, so the module did draw)
expect display.dark.1290.100.372.214 is 79608
# The title bar between the name and the minimize button, the page above, left of, right of and below the frame
# (where the popup was, too) are as they were: nothing dark.
expect display.dark.1430.36.110.44 is 0
expect display.dark.1276.2.400.24 is 0
expect display.dark.1000.30.270.220 is 0
expect display.dark.1680.30.20.180 is 0
expect display.dark.1280.334.390.50 is 0
# A tap in the body reaches the module's region; the status line says which region the last tap hit.
expect editor.widget.0.hits is 2
tap panel widget.0.hit.7
wait 300
expect editor.widget.0.status is 7
# A tap in the frame's margin, beside the body, is in none of the module's regions.
tap 1281 323
wait 300
expect editor.widget.0.status is /
# The whole screen drawn again (a new page) draws the frame again, and still only its body.
tap panel next
wait 600
expect editor.page is 2
expect display.dark.1290.100.372.214 is 79608
expect display.dark.1430.36.110.44 is 0
expect display.dark.1000.30.270.220 is 0
expect display.dark.1280.334.390.50 is 0
expect display.dark.200.400.900.40 is 0
shot 02-wide-new-page
tap panel widget.0.close
wait 300
expect editor.widgets is 0
# Plain is the same module in a package whose manifest declares no capability. Its frame opens at the same place;
# nothing the module draws is shown, it has no hit region, and a tap leaves no state for the status line to say.
tap panel widgets
wait 300
tap panel widgets.test.host.plain
wait 400
expect editor.widgets is 1 test.host.plain
expect editor.widget.0.rect is 1276 28 400 300
expect editor.widget.0.error is
expect editor.widget.0.hits is 0
expect display.dark.1290.100.372.214 is 0
shot 03-plain
tap 1476 207
wait 300
expect editor.widget.0.status is
expect editor.widget.0.hits is 0
tap panel widget.0.close
wait 300
expect editor.widgets is 0
# Many draws 513 rectangles in a pass, one more than a pass may hold: the pass is stopped as one that used up its
# instructions is, none of it is drawn, and the frame says that the widget stopped.
tap panel widgets
wait 300
tap panel widgets.test.host.many
wait 400
expect editor.widgets is 1 test.host.many
expect editor.widget.0.error is The widget stopped: all fuel consumed by WebAssembly
expect editor.widget.0.hits is 0
expect editor.widget.0.text is
shot 04-many
# (the frame is still a frame: it closes)
tap panel widget.0.close
wait 300
expect editor.widgets is 0
# Changed is a package whose module is not the file its install recorded (the index holds another SHA-256): the
# file is not loaded, and the frame says that it could not be read.
tap panel widgets
wait 300
tap panel widgets.test.host.changed
wait 400
expect editor.widgets is 1 test.host.changed
expect editor.widget.0.error is The widget's artifact could not be read.
expect editor.widget.0.hits is 0
# (its module is Wide's, which fills the body with black when it runs)
expect display.dark.1290.100.372.214 not 79608
shot 05-changed
tap panel widget.0.close
wait 300
expect editor.widgets is 0
# Small is installed with a smallest size of 120x80, less than a frame's title bar and margins take. Dragged by
# its grip as small as it goes, the frame stops at 120x120, the smallest there is, and still has a body: 92x34 at
# 1470,100, which the module fills (checked left of the corner the frame's own grip is drawn on).
tap panel widgets
wait 300
tap panel widgets.test.host.small
wait 400
expect editor.widgets is 1 test.host.small
expect editor.widget.0.rect is 1456 28 220 160
drag touch 1649 161 1300 40 8
wait 400
expect editor.owner is none
expect editor.widget.0.rect is 1456 28 120 120
expect editor.widget.0.error is
expect editor.widget.0.hits is 2
expect display.dark.1470.100.60.34 is 2040
shot 06-small
tap panel widget.0.close
wait 300
expect editor.widgets is 0
# Long is a package whose id is as long as an id may be, 128 bytes. A scenario line cannot name a row that long,
# so its row, the sixth of the popup, is tapped where it is. Its frame opens, draws its button and is left open:
# z_widget_host_kept starts Grimoire again on this data and finds it.
tap panel widgets
wait 300
shot 07-popup
tap 1404 2246
wait 400
expect editor.widgets.open is 0
expect editor.widgets is 1 test.host.long.0123456789-abcdefghij.0123456789-abcdefghij.0123456789-abcdefghij.0123456789-abcdefghij.0123456789-abcdefghij.01z
expect editor.widget.0.rect is 1276 28 400 300
expect editor.widget.0.error is
expect editor.widget.0.text is hello
drag touch 1400 60 900 700 8
wait 400
expect editor.owner is none
expect editor.widget.0.rect is 776 668 400 300
shot 08-long
wait 1800
quit
