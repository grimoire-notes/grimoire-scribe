# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: ink_edges=solid
# settings: status_size=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script
# copy: tests/fixtures/widgetruns .
# A solid word written beside a widget that asked for nothing (2.3: Ink edges at Solid, #320, and the refreshes
# nobody asked for, issue #280), with Ticker of tests/fixtures/widgetruns, which asks to be drawn again a second after
# each pass. The frame's redraw comes due while the pen is writing and waits for it: once the pen has left it goes
# out, the frame's rectangle with the hint (display_unasked, the runner's display.unasked). Then the word's settle
# finds the panel showing the page already: it sends nothing (editor.settles_quiet), and says nothing to the platform
# either, since app_flush gives its word only with the present it is for. So the page turn after it, a refresh
# something asked for, goes without the hint, and the frame's next redraw has it again. App time is the script's
# (GRIMOIRE_SIM_CLOCK=script): each `wait` moves it on by exactly its milliseconds.
# (the status bar's clock held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
tap library add
tap library add.new-notebook
wait 100
tap library paper.4
wait 100
tap library create
wait 300
expect screen is editor
expect editor.paper is 4
pentap panel tool-picker
pentap 872 561
expect editor.tool is ballpoint
# (a first stroke makes Undo available, which the panel shows with a refresh of its own)
pen line 300 400 700 420 20 0.6 0.6
pen leave
wait 2000
expect editor.strokes is 1
expect editor.settles_quiet is 1
tap panel widgets
wait 300
tap panel widgets.test.runs.ticker
wait 400
expect editor.widgets is 1 test.runs.ticker
expect editor.widget.0.rect is 1456 106 220 160
waitfor editor.widget.0.status is 2
# The word, begun 700 ms after that pass: the third is due while the pen is on the page, and waits for it
wait 700
expect display.presents not -1
expect display.unasked not -1
pen down 300 900 0.3 0 0
pen move 340 860 0.4 0 0
pen move 380 840 0.5 0 0
pen move 420 860 0.6 0 0
wait 150
pen move 440 920 0.7 0 0
pen move 470 980 0.8 0 0
pen move 520 960 0.8 0 0
wait 150
pen move 560 900 0.7 0 0
pen move 600 860 0.6 0 0
pen move 660 880 0.6 0 0
wait 150
pen move 700 940 0.7 0 0
pen move 760 960 0.9 0 0
pen move 820 900 0.9 0 0
expect display.last_fast is ink
expect display.stale.200.700.1000.400 is 0
expect editor.widget.0.status is 2
expect display.presents has fast
expect display.unasked is 0
pen up 830 890
pen leave
# The frame's redraw, which waited for the pen: its rectangle, with the hint
expect display.presents has ui 1456 106 220 160
expect display.unasked is 1
expect editor.widget.0.status is 3
# The word's settle, 150 ms after the pen left: nothing sent, nothing said
wait 400
expect editor.strokes is 2
expect editor.settles_quiet is 2
expect editor.settles_sent is 0
expect display.presents is
expect display.unasked is 0
expect display.stale.200.700.1000.400 is 0
expect editor.widget.0.status is 3
# A key's next page (on the last page it adds one): asked for, so no hint. The new page's drawing draws the frame
# too, a pass of the module's in the refresh the page turn asked for.
key 106 0
wait 300
expect editor.page is 2
expect editor.widget.0.status is 4
expect display.unasked is 0
expect display.presents not -1
# And the frame's next redraw has the hint again
waitfor editor.widget.0.status is 5
expect display.presents is ui 1456 106 220 160
expect display.unasked is 1
shot 01-word-and-ticker
quit
