# settings: status_on=0
# settings: status_in_notebooks=0
# rust-only
# The status bar's hidden state is saved: a start with status_on=0 (and notebooks off) draws no bar
# anywhere, while the top strip still opens quick settings.
wait 500
expect screen is library
expect settings.status_on is 0
expect settings.status_in_notebooks is 0
expect status.visible is 0
expect status.zone is 0 0 0 0
expect quick is 0
shot 00-hidden-at-start
touch down 930 40
touch move 930 140
wait 150
expect quick is 1
touch up 930 140
wait 100
expect quick.rect is 0 0 1860 556
shot 01-quick-with-bar-hidden
tap panel quick.close
wait 200
expect quick is 0
quit
