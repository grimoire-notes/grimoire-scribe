# settings: status_mode=1
# settings: status_size=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitpanel .
# A frame's contact the platform cancels is no tap (issue #272), with Tap widget of tests/fixtures/kitpanel
# (rust/tests/kitfix: `make kit-fixtures`), which has widget_tap and no kit_event and counts widget_tap's calls (n).
# A finger on its body, then a second finger on the page: touchgest makes the two a gesture and cancels the first,
# whose lift calls no widget_tap. A plain tap after it calls widget_tap once.
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
tap panel widgets.test.kit.tapw
wait 400
expect editor.widget.0.rect is 776 106 900 1100
expect editor.widget.0.status is p001 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
finger 1 down 1300 800
frame
wait 100
finger 2 down 1300 1700
frame
wait 100
finger 1 up 1300 800
finger 2 up 1300 1700
frame
wait 300
expect editor.owner is none
expect editor.widget.0.status is p001 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
tap 1300 800
wait 300
expect editor.widget.0.status is p002 n001 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
quit
