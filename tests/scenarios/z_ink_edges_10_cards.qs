# settings: status_mode=1
# env: GRIMOIRE_BG_DELAY_MS=3000
# Ink edges (2.3, #320): the Library's cards follow a change of the setting without their notebooks being opened. A
# card drawn with Smooth stays as it is while the background job (slowed here, as on the Kindle) draws it again with
# Solid; then the Library shows the new one (darker: the marker's grays are black), which is the page as Solid draws
# it. Back to Smooth, likewise (the job draws the card as a closing notebook does, without the editor's kept lines,
# hence a level lighter than the first).
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
pentap 1504 561
expect editor.tool is marker
pen line 300 400 1500 410 30 0.6 0.6
pen line 300 480 1500 490 30 0.6 0.6
pen line 300 560 1500 570 30 0.6 0.6
pen line 300 640 1500 650 30 0.6 0.6
pen line 300 720 1500 730 30 0.6 0.6
pen line 300 800 1500 810 30 0.6 0.6
pen line 300 880 1500 890 30 0.6 0.6
pen line 300 960 1500 970 30 0.6 0.6
pen line 300 1040 1500 1050 30 0.6 0.6
pen line 300 1120 1500 1130 30 0.6 0.6
pen line 300 1200 1500 1210 30 0.6 0.6
pen line 300 1280 1500 1290 30 0.6 0.6
pen line 300 1360 1500 1370 30 0.6 0.6
pen line 300 1440 1500 1450 30 0.6 0.6
pen line 300 1520 1500 1530 30 0.6 0.6
pen line 300 1600 1500 1610 30 0.6 0.6
pen line 300 1680 1500 1690 30 0.6 0.6
pen line 300 1760 1500 1770 30 0.6 0.6
pen line 300 1840 1500 1850 30 0.6 0.6
pen line 300 1920 1500 1930 30 0.6 0.6
pen leave
wait 2000
pentap panel library
wait 600
expect screen is library
waitbg
wait 300
expect library.card.1 is thumb
expect library.card.1.luma is 244
expect library.cards_redrawn is 0
shot 01-card-smooth
tap library settings
wait 100
tap library settings.cat.3
wait 100
tap library ink-edges
wait 100
tap library ink-edges.1
wait 100
expect settings.ink_edges is solid
tap library done
wait 300
expect library.card.1.luma is 244
waitbg
wait 300
expect library.cards_redrawn is 1
expect library.card.1.luma is 239
shot 02-card-solid
tap library card.1
wait 700
expect screen is editor
expect editor.card_matches_render is 1
pentap panel library
wait 600
expect screen is library
tap library settings
wait 100
tap library settings.cat.3
wait 100
tap library ink-edges
wait 100
tap library ink-edges.0
wait 100
expect settings.ink_edges is smooth
tap library done
wait 300
waitbg
wait 300
expect library.cards_redrawn is 2
expect library.card.1.luma is 245
tap library card.1
wait 700
expect editor.card_matches_render is 1
quit
