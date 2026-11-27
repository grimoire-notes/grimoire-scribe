# rust-only
# generate: tests/home_fixture.py
# settings: status_size=0
# settings: recent_apps=pkg:com.example.calculator,pkg:com.example.chess,pkg:com.example.weather,pkg:com.example.wordsearch
# env: TZ=UTC
# The home as the owner chose it (#332), on a populated Library (tests/home_fixture.py: five installed apps, three
# folders, one with an icon and one with a picture, and notebooks with ink on different papers): no bat and nothing
# on the home that opens What's new (the Apps page's foot button does); Search, then Settings at the far right;
# the app row in even squares with Show All its last, a name too long for one line (Sudoku Puzzles) on two; the
# title rule, the section break, the light rule and the foot rule, every one outside every hit; the page arrows and
# their line on the + button's centre line; Classic's + a rounded rectangle. Gothic and Classic, upright and
# sideways, in a folder, the Apps page, at the small status bar and then the large one. Each notebook on the first
# screen, and School's, is opened and left once, so that its card is drawn from its page. The clock is held still,
# so the shots are the same on every run.
clock 1795772460
wait 3600
expect screen is library
expect library.items is 10
tap library notebook.book-notes
wait 600
tap panel library
wait 600
waitbg
tap library notebook.daily-planner
wait 600
tap panel library
wait 600
waitbg
tap library notebook.garden-plan
wait 600
tap panel library
wait 600
waitbg
tap library notebook.ideas
wait 600
tap panel library
wait 600
waitbg
tap library notebook.meeting-notes
wait 600
tap panel library
wait 600
waitbg
tap library folder.School
wait 300
tap library notebook.algebra
wait 600
tap panel library
wait 600
waitbg
tap library notebook.biology
wait 600
tap panel library
wait 600
waitbg
tap library notebook.history-essay
wait 600
tap panel library
wait 600
waitbg
expect library.folder is School
tap library back
wait 300
waitfor library.apps.icons is 5
waitbg
wait 300
expect library.folder is
# Gothic, upright, small status bar: the header holds the title, then Search and Settings at the far right, and
# nothing that opens What's new; the row's squares, one step apart, Show All the last at the right margin; the four
# marks top to bottom, none crossing a hit; the arrows and their line on the + button's centre line.
expect library.home.header is title search settings
expect library.ui_has.about is 0
expect library.ui_at.1654.147 is search
expect library.ui_at.1762.147 is settings
expect library.ui_at.114.153 is
expect library.ui_at.400.153 is
expect library.home.squares is 51 270 176 176, 277 270 176 176, 503 270 176 176, 729 270 176 176, 955 270 176 176, 1181 270 176 176, 1407 270 176 176, 1633 270 176 176
expect library.home.step is 226
expect library.home.labels is 1 1 1 1 1 1 1 1
expect library.ui_at.139.358 is app.files
expect library.ui_at.365.358 is app.store
expect library.ui_at.591.358 is app.share
expect library.ui_at.817.358 is app.com.example.calculator
expect library.ui_at.1495.358 is app.com.example.wordsearch
expect library.ui_at.1721.358 is apps
expect library.ui_at.1721.500 is apps
expect library.home.marks is title 227 11, break 534 32, light 703 2, foot 2235 11
expect library.home.marks_in_hits is 0
expect library.home.foot_axis is 2358 2358 2358 2358
expect library.home.fab is 1666 2286 144 144 cut 22
shot 01-gothic
# A tap on the title, or where the bat stood, opens nothing.
tap 114 153
wait 200
expect library.sheet is none
tap 400 153
wait 200
expect library.sheet is none
# The arrows turn the page where they are drawn.
tap library next
wait 300
expect library.page is 2
expect library.home.foot_axis is 2358 2358 2358 2358
tap library prev
wait 300
expect library.page is 1
rotate 1
wait 300
expect library.home.header is title search settings
expect library.home.squares is 52 270 176 176, 272 270 176 176, 492 270 176 176, 712 270 176 176, 932 270 176 176, 1152 270 176 176, 1372 270 176 176, 1592 270 176 176, 2252 270 176 176
expect library.home.step is 220
expect library.home.labels is 1 1 1 1 1 1 1 2 1
expect library.home.marks is title 227 11, break 534 32, light 703 2, foot 1615 11
expect library.home.marks_in_hits is 0
expect library.home.foot_axis is 1738 1738 1738 1738
expect library.home.fab is 2286 1666 144 144 cut 22
shot 03-gothic-sideways
rotate 0
wait 300
tap library folder.School
wait 300
expect library.home.header is back title search settings
expect library.ui_at.98.153 is back
expect library.ui_has.about is 0
expect library.home.marks is title 227 11, break 534 32, light 703 2, foot 2235 11
expect library.home.marks_in_hits is 0
expect library.home.foot_axis is 2358 2358 2358 2358
shot 04-gothic-folder
tap library back
wait 300
# The Apps page: the row's squares in its columns, and What's new at its foot.
tap library apps
wait 300
expect library.sheet is 301
expect library.ui_at.139.334 is app.files
expect library.ui_at.1721.334 is app.com.example.sudoku
expect library.ui_at.1721.480 is app.com.example.sudoku
shot 05-gothic-apps
tap library app.whats-new
wait 300
expect library.sheet is about
expect library.about.note is 2.3.0-beta.1
tap library about.close
wait 300
expect library.sheet is none
# Classic: the same places, single rules and a plain line at the main split, and the + a rounded rectangle.
tap library settings
wait 300
tap library settings.cat.1
wait 100
tap library theme
wait 300
expect settings.theme is classic
tap library done
wait 300
expect library.home.header is title search settings
expect library.home.squares is 51 270 176 176, 277 270 176 176, 503 270 176 176, 729 270 176 176, 955 270 176 176, 1181 270 176 176, 1407 270 176 176, 1633 270 176 176
expect library.home.marks is title 231 3, break 549 2, light 703 2, foot 2243 3
expect library.home.marks_in_hits is 0
expect library.home.fab is 1666 2286 144 144 round 24
expect library.home.foot_axis is 2358 2358 2358 2358
shot 02-classic
rotate 1
wait 300
expect library.home.labels is 1 1 1 1 1 1 1 1 1
expect library.home.marks is title 231 3, break 549 2, light 703 2, foot 1623 3
expect library.home.marks_in_hits is 0
expect library.home.fab is 2286 1666 144 144 round 24
expect library.home.foot_axis is 1738 1738 1738 1738
shot 06-classic-sideways
rotate 0
wait 300
tap library folder.School
wait 300
expect library.home.header is back title search settings
expect library.home.marks_in_hits is 0
tap library back
wait 300
tap library apps
wait 300
shot 07-classic-apps
tap library apps.close
wait 300
expect library.sheet is none
# The large status bar: everything under the header moves down with it, the cards are still two rows, and the
# foot does not move.
tap library settings
wait 300
tap library settings.cat.2
wait 100
tap library status-bar
wait 300
tap library display.size
wait 200
tap library display.size
wait 200
expect settings.status_size is 2
tap library display.done
wait 300
expect library.sheet is none
expect library.pages is 2
expect library.home.squares is 51 290 176 176, 277 290 176 176, 503 290 176 176, 729 290 176 176, 955 290 176 176, 1181 290 176 176, 1407 290 176 176, 1633 290 176 176
expect library.home.marks is title 251 3, break 569 2, light 723 2, foot 2243 3
expect library.home.marks_in_hits is 0
expect library.home.foot_axis is 2358 2358 2358 2358
shot 08-classic-large
tap library settings
wait 300
tap library settings.cat.1
wait 100
tap library theme
wait 300
expect settings.theme is gothic
tap library done
wait 300
expect library.home.squares is 51 290 176 176, 277 290 176 176, 503 290 176 176, 729 290 176 176, 955 290 176 176, 1181 290 176 176, 1407 290 176 176, 1633 290 176 176
expect library.home.marks is title 247 11, break 554 32, light 723 2, foot 2235 11
expect library.home.marks_in_hits is 0
expect library.home.foot_axis is 2358 2358 2358 2358
shot 09-gothic-large
rotate 1
wait 300
expect library.home.marks is title 247 11, break 554 32, light 723 2, foot 1615 11
expect library.home.marks_in_hits is 0
expect library.home.foot_axis is 1738 1738 1738 1738
shot 10-gothic-large-sideways
rotate 0
wait 300
quit
