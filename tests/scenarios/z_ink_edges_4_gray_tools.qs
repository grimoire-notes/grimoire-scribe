# settings: status_mode=1
# settings: ink_edges=solid
# Ink edges at Solid (2.3, #320) leaves the other tools as they were: a pencil line and a highlight stay gray while they
# are drawn and settle in grays after (the pause sends a refresh: editor.settles_sent), and a highlight and a pencil
# line across a solid ballpoint word leave every black pixel of it black (display.dark over the word, the same before
# and after). The page is the strokes drawn afresh, with the solid word over the others' grays.
# rust-only
wait 3600
clock 1700000000
tap library add
tap library add.new-notebook
wait 100
tap library paper.4
wait 100
tap 1564 2254
wait 400
expect screen is editor
pentap panel tool-picker
pentap 872 561
expect editor.tool is ballpoint
pen line 300 600 1300 620 40 0.6 0.6
pen leave
wait 2000
expect editor.settles_quiet is 1
expect display.stale.200.500.1300.200 is 0
expect display.dark.250.560.1100.100 is 5224
# A highlight along the word: gray, settled in grays; the word stays as black as it was
pentap panel tool-picker
pentap 1504 727
expect editor.tool is highlighter
pen line 280 610 1320 610 40 0.8 0.8
pen leave
wait 2000
expect editor.settles_sent is 1
expect display.stale.200.400.1300.400 is 0
expect display.dark.250.560.1100.100 is 5224
# A pencil line on its own, on blank paper: its grays reach the panel dithered while it is drawn (display.stale over
# it, read while the pen is still down: nothing settles under a stroke, however late the loop is), and its pause
# settles it in grays
pentap panel tool-picker
pentap 872 395
expect editor.tool is pencil
pen down 300 1000 0.7 40 30
pen move 341 1000 0.7 40 30
pen move 383 1001 0.7 40 30
pen move 425 1002 0.7 40 30
pen move 466 1003 0.7 40 30
pen move 508 1004 0.7 40 30
pen move 550 1005 0.7 40 30
wait 30
pen move 591 1005 0.7 40 30
pen move 633 1006 0.7 40 30
pen move 675 1007 0.7 40 30
pen move 716 1008 0.7 40 30
pen move 758 1009 0.7 40 30
pen move 800 1010 0.7 40 30
wait 30
pen move 841 1010 0.7 40 30
pen move 883 1011 0.7 40 30
pen move 925 1012 0.7 40 30
pen move 966 1013 0.7 40 30
pen move 1008 1014 0.7 40 30
pen move 1050 1015 0.7 40 30
wait 30
pen move 1091 1015 0.7 40 30
pen move 1133 1016 0.7 40 30
pen move 1175 1017 0.7 40 30
pen move 1216 1018 0.7 40 30
pen move 1258 1019 0.7 40 30
expect display.stale.200.900.1300.250 not 0
pen up 1300 1020
pen leave
wait 2000
expect editor.settles_sent is 2
expect display.stale.200.900.1300.250 is 0
# A pencil line across the word
pen line 700 450 720 780 30 0.7 0.7 40 30
pen leave
wait 2000
expect editor.settles_sent is 3
expect display.stale.200.400.1300.800 is 0
expect editor.live_matches_render is 1
shot 01-gray-over-solid
quit
