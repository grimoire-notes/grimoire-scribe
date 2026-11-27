# settings: status_mode=1
# settings: status_on=1
# settings: status_in_notebooks=1
# env: TZ=UTC GRIMOIRE_SIM_NO_LIGHT=1
# rust-only
# Quick settings on a platform without a front light (light_available() false, docs/SIM.md): its
# controls are mostly the light's, so there is no panel. The pull from the top opens nothing, the
# screen under it does not see it, and nothing reads or sets the light. In a notebook a tap in the
# top strip still hides and shows the status bar.
wait 3800
clock 1700000000
battery 87
wait 100
expect screen is library
expect status.visible is 1
expect quick is 0
expect light.intensity is 12
# On the home screen: the pull from the bar opens nothing, however far it goes.
touch down 930 40
touch move 930 120
wait 150
expect quick is 0
touch move 930 600
wait 150
expect quick is 0
touch up 930 600
wait 150
expect quick is 0
expect screen is library
expect library.sheet is none
expect light.intensity is 12
shot 00-home-after-pull
# A tap on the bar on the home screen leaves it shown, as with a light.
tap 930 40
wait 200
expect status.visible is 1
expect quick is 0
# Into a notebook: the pull opens nothing over the page and draws nothing on it.
tap library add
wait 100
tap library add.new-notebook
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
expect status.visible is 1
expect editor.strokes is 0
touch down 930 40
touch move 930 120
wait 150
expect quick is 0
touch move 930 600
wait 150
touch up 930 600
wait 200
expect quick is 0
expect screen is editor
expect editor.modal is none
expect editor.strokes is 0
expect status.visible is 1
expect light.intensity is 12
shot 01-notebook-after-pull
# A tap in the top strip still hides the notebook's bar, and another brings it back.
tap 930 40
wait 300
expect settings.status_on is 0
expect status.visible is 0
expect quick is 0
tap 930 40
wait 300
expect settings.status_on is 1
expect status.visible is 1
expect quick is 0
quit
