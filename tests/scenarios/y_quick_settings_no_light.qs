# settings: status_mode=1
# settings: status_on=1
# env: TZ=UTC GRIMOIRE_SIM_NO_LIGHT=1
# rust-only
# Quick settings on a platform without a front light (light_available() false, docs/SIM.md): the pull
# from the top still opens the panel, with Settings and Rotation lock, and none of the light's controls
# (no Brightness or Warmth row, no Auto brightness). The panel is only as tall as what it holds, and
# nothing reads or sets the light.
wait 3800
clock 1700000000
battery 87
wait 100
expect screen is library
expect status.visible is 1
expect quick is 0
touch down 930 40
touch move 930 120
wait 150
expect quick is 1
touch up 930 120
wait 150
expect quick is 1
# The title, Close and two buttons: the panel ends under the buttons (with a light it is 556 tall).
expect quick.rect is 0 78 1860 284
expect quick.brightness is -1
expect quick.warmth is -1
expect quick.auto is -1
shot 00-quick-without-light
# Rotation lock turns on and off from here, as with a light.
expect settings.rotation_lock is 0
tap panel quick.rotate
wait 150
expect settings.rotation_lock not 0
tap panel quick.rotate
wait 150
expect settings.rotation_lock is 0
expect quick is 1
# Where the Brightness row would be is outside the panel now: a tap there closes it, and the light
# is not touched (the sim keeps its level, 12, behind the switch).
expect light.intensity is 12
tap 930 420
wait 200
expect quick is 0
expect light.intensity is 12
# The Settings button closes the panel and opens Settings.
touch down 930 40
touch move 930 140
wait 150
expect quick is 1
touch up 930 140
wait 100
expect quick.rect is 0 78 1860 284
tap panel quick.settings
wait 300
expect quick is 0
expect library.sheet is settings
