# The Seyes notebook y_rulings_1_sideways made on an 1860x2480 screen, opened on a screen of 1920x2560 and
# turned sideways (issue #308): the whole page rests at 0.77 (1920 / 2480), its paper drawn at the screen's
# scale with every line as at 100%, the ink written on the other screen over it.
# root: y_rulings_1_sideways
# env: GRIMOIRE_SIM_SIZE=1920x2560
# rust-only
wait 300
expect size is 1920x2560
expect library.items is 1
# The notebook, the first card, with a finger
tap 336 622
wait 500
expect screen is editor
expect editor.paper is 20
expect editor.page_size is 1860x2480
expect editor.strokes is 4
rotate 1
wait 1500
expect size is 2560x1920
expect editor.scale is 0.77
expect editor.zoomed is 0
expect editor.screen_composed is 1
expect editor.layers is ok
shot 01-sideways
rotate 0
wait 1500
expect editor.screen_composed is 1
shot 02-upright
quit
