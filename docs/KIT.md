# The package kit: `grimoire_kit`

The package kit is a second import module, `grimoire_kit`, that both of Grimoire's WebAssembly hosts link: a
widget's frame ([WIDGETS.md](WIDGETS.md)) and an app ([APPS.md](APPS.md)). Its calls mean the same in both, so a
widget and an app draw one set of elements: the theme's standard sizes, text by role, and the theme's elements,
which take the theme's standard size when a call gives them none. A package's screen is then drawn as
Grimoire's own are, pixel for pixel, in Gothic and Classic and in each font of Settings > Font.

Each host's own module, `grimoire`, stays as it is: the same calls, signatures and meaning. A module may import
from both; its commands are drawn in the order it called them. The kit's code is `rust/src/kithost.rs`.

## Levels

The kit is versioned by level. A level's calls, metric ids, roles and flags never change their signature or
meaning; a later level adds new ones. An id, role or flag a level does not have is answered in a defined way
(below): -1 for a metric id, BODY for a role, plain for a style.

| level | Grimoire | calls |
|---|---|---|
| 1 | 2.3.0 | the calls on this page |

This page lists the calls of level 1 that Grimoire has now. A module that imports a call of `grimoire_kit` that
this Grimoire does not have is not started: a widget's frame says `This widget needs a newer Grimoire
(grimoire_kit.<call>).` in place of its body, and an app does not open and a toast says `This app needs a newer
Grimoire (grimoire_kit.<call>).` The refusal names a call only when the name has a call's shape, 1 to 64 bytes of
`a` to `z`, `0` to `9` and `_` (every call of every level has one); for any other name it says `grimoire_kit` alone.
A module that imports a call of this level with another signature, or as something other than a function, is not
started either: the frame or the toast gives the interpreter's message (`The widget could not start: ...`). A
package's `min_grimoire_version` stays the install gate.

## Units

The unit is the screen pixel, as for the hosts' own calls: the body's size, a place, a size. Every place is
relative to the body's top left. Sizes come from the theme: `metric` answers the standard sizes Grimoire's
screens use, already at the panel's scale, and an element given a width or a height of 0 or less takes the
standard one the table of Elements gives (`fill` and `frame` given a height of 0 or less draw nothing). Each element returns the extent it took (a row its height, a button its
width), so a module lays its screen out with a cursor:

```text
y = metric(MARGIN)
y += text(x, y, w, 0, title, len, TITLE, 0) + metric(GAP)
y += row(1, x, y, w, 0, icon, ilen, label, llen, 0, -1) + metric(GAP)
```

## Calls (level 1)

Every value is an i32 unless marked. A string is `ptr, len` (UTF-8 in the module's memory); a buffer to write
into is `ptr, cap`, and the call returns the bytes written, never more than `cap` or than the memory holds there.
An `id` of 0 or more makes an element a hit region with that id; -1 makes none.

```text
level() -> i32
version(ptr, cap) -> i32
metric(id) -> i32
clock_ms() -> i64

text(x, y, w, h, ptr, len, role, flags) -> i32                    the height it took
text_lines(x, y, w, h, ptr, len, role, flags, first_line) -> i32  the text's whole count of lines
measure_width(ptr, len, role) -> i32
measure_height(ptr, len, role, w) -> i32

button(id, x, y, w, h, label_ptr, label_len, icon_ptr, icon_len, style) -> i32        its width
icon_button(id, x, y, w, h, icon_ptr, icon_len, style) -> i32                          its width
row(id, x, y, w, h, icon_ptr, icon_len, label_ptr, label_len, style, switch) -> i32    its height
chip(id, x, y, w, h, label_ptr, label_len, selected) -> i32                            its width
field(id, x, y, w, h, icon_ptr, icon_len, text_ptr, text_len, hint_ptr, hint_len, flags) -> i32   its height
select(id, x, y, w, h, caption_ptr, caption_len, value_ptr, value_len) -> i32          its height
checkbox(id, x, y, w, h, label_ptr, label_len, checked) -> i32                         its height
card(x, y, w, h, style)
divider(x, y, w)
icon(x, y, w, h, name_ptr, name_len, tone)
fill(x, y, w, h, gray)
frame(x, y, w, h, gray, filled, thickness)
line(x0: f32, y0: f32, x1: f32, y1: f32, width: f32, gray)
clip(x, y, w, h)
clip_clear()
hit(id, x, y, w, h)
list_page(n, page, area_h, pref_h, min_h) -> i64
pager(prev_id, next_id, x, y, w, page, pages) -> i32                                  its height

event_points(ptr, cap) -> i32                                     the points it wrote
key_char(code, mods) -> i32                                       the character, 0 for none

export (optional): kit_event(kind, id, a, b, c) -> i32            nonzero: the picture changed
```

- `level` is 1.
- `version` writes Grimoire's version (`2.3.0`, `2.3.0-beta.1`), at most 32 bytes.
- `metric` answers a standard size or value (Metrics), or -1 for an id this level does not have.
- `clock_ms` is the app's clock in milliseconds: what `request_redraw` counts in, and what a scenario's `clock`
  line holds still. It counts from an arbitrary start, and wraps after about 49 days.
- `measure_width` is one line's width in a role; `measure_height` the height of the text broken into lines for a
  width `w` (0 or less: the body's width), as `text` with WRAP breaks it.
- `list_page` is the layout Settings pages its lists with (`ui_row_page`): for a list of `n` rows in an area
  `area_h` high, rows `pref_h` high when they fit and down to `min_h` (no less than 72) when the list is long, it
  answers the rows a page holds, the rows' height, the gap between them and the pages, each in 16 bits:
  `rows | height << 16 | gap << 32 | pages << 48`. A paged list keeps 110 px at its foot for the pager. `page` is
  not used in level 1 (a module clamps it to the pages itself).
- `pager` draws Settings' arrows and count under a paged list: the previous page's arrow at `x`, the next's at the
  right end, each 130 x 90, the count `page + 1 / pages` between them; an arrow with no page that way is drawn
  disabled and is no hit region. It draws nothing and returns 0 for one page or none.
- `event_points`, `key_char` and the export `kit_event`: [Events](#events).

## Metrics

Pixels at 300 dpi, the size on every supported panel (Gothic / Classic where they differ). Each size a screen of
Grimoire's draws at is the function that screen reads (`globalui.rs`, `ui.rs`), so the two cannot differ; FIELD_H,
which no screen draws yet, is the kit's own.

| id | name | value | from |
|---|---|---|---|
| 1 | TEXT_BODY | 44 | the text size |
| 2 | TEXT_SMALL | 34 | the small size |
| 3 | TEXT_TITLE | 64 | the title size |
| 4 | TEXT_DISPLAY | 112 / 70 | a screen's title (the title size x 1.75 in Gothic's blackletter, x 1.1 in Classic) |
| 5 to 9 | LINE_BODY, LINE_SMALL, LINE_TITLE, LINE_DISPLAY, LINE_MONO | a line of that role: the face's ascent and descent in the font in use | |
| 10 | ROW_H | 118 | Settings' rows |
| 11 | ROW_MIN_H | 90 | Settings' rows when a list is tight |
| 12 | BUTTON_H | 96 | the Package Store's action buttons (`gui_button_row`, which the kit's button with an icon and a label is) |
| 13 | CHIP_H | 88 | the home's and the Package Store's filter chips |
| 14 | FIELD_H | 112 | no screen of Grimoire draws a text field yet: the height of the package manager's action rows, as the kit's design gives it |
| 15 | ICON | 60 | a row's icon |
| 16 | ICON_BUTTON | 96 | the header's round buttons |
| 17 | TOUCH_MIN | 90 | the smallest control Grimoire draws |
| 18 | HEADER_H | 150 | the header under the status bar |
| 19 | SELECT_H | 88 | the home's Sort |
| 20 | MARGIN | 50 | a screen's margin |
| 21 | GAP | 24 | the room between two elements |
| 22 | ROW_GAP | 16 | the room between two rows of a list |
| 23 | PAD | 30 | the room inside a card |
| 24 | RADIUS | 16 / 22 | a card's corners (`card`, `row` and `frame` draw at it) |
| 25 | LINE | 3 | a card's outline |
| 30 | SCALE_MILLI | 1000 | the UI scale times 1000 |
| 31 | DPI | 300 | the panel's |
| 32 | THEME | 0 Gothic, 1 Classic | |
| 33 | FONT | Classic: 0 Terminess, 1 Atkinson Hyperlegible Next, 2 Noto Sans, 3 Literata, 4 the reader's own font; Gothic: 5 | Settings > Font |
| 34 | STATUS_BAR_H | 78 at the medium size (64 small, 92 large); an app that hides the bar: 0 | the status bar |
| 35 | KEYBOARD_TOP | -1: no keyboard of the kit's is open in this level | |
| 36 | BODY_W | the width of the body the call draws on, from the module's start on: a widget's frame's body, an app's under the status bar its manifest asks for; then a drawing pass's, and the last pass's in another call | |
| 37 | BODY_H | its height, the same way | |
| 40 | GRAY_INK | 0 | |
| 41 | GRAY_PAPER | 255 | |
| 42 | GRAY_MUTED | 100 | a field's hint |
| 43 | GRAY_DISABLED | 170 | a disabled control |
| 44 | GRAY_RULE | 140 | a field's and a select's outline in Classic |

## Text

A text is given a role, never a pixel size or a face. The role picks the theme's face and size:

| role | name | face, size |
|---|---|---|
| 0 | BODY | regular, 44 |
| 1 | SMALL | regular, 34 |
| 2 | LABEL | bold, 44 (Settings' category titles) |
| 3 | CAPTION | bold, 34 |
| 4 | TITLE | the title face, 64 |
| 5 | MONO | fixed, 44 |
| 6 | MONO_SMALL | fixed, 34 |
| 7 | DISPLAY | a screen's title: Gothic's blackletter at 112, Classic's title face at 70 |

A role outside 0 to 7 is BODY.

`flags`: bits 0 and 1 the alignment (0 left, 1 centred, 2 right; 3 is left); bit 2 (4) WRAP; bit 3 (8) FIT; bits 4
and 5 the tone (0 ink, 1 muted 100, 2 disabled 170, 3 paper 255, for text on a filled element).

- A text is one line, vertically centred in its rectangle, and cut short with `...` when it is wider than its
  room, as Grimoire's labels are. A right-aligned text too wide for its room is drawn as a left-aligned one is.
- FIT is Grimoire's label rule: the role's size, else a smaller one down to the small size that holds it on one
  line, else two lines at the small size. Given no height it takes one line, or two when it takes two.
- WRAP breaks the text into lines: at its own line breaks (`\n`), then as many whole words as fit the width on each
  line, and a word wider than the width is broken between its characters. With both WRAP and FIT, WRAP is taken. The lines are centred as a block in
  the rectangle, as many as it is high for; when more follow, the last line shown holds the rest, cut short. Given
  no height it takes as many lines as it needs, and `text` returns their height.
- `text_lines` breaks the text the same way and draws, from the top of its rectangle, the lines from `first_line`
  (0 is the first) that its height holds, and returns how many lines the whole text has: a module pages a long
  text so, as the license viewer does. Given no height it takes the rest of the body below `y`. FIT has no effect
  on it.
- A package's text is not reported to Grimoire's translation fit check (a scenario's `GRIMOIRE_CHECK_FIT`).

## Elements

Each is the element Grimoire's own screens draw, in the theme in use. `style` is 0 plain, 1 selected, 2 disabled,
3 primary; another number is plain. `switch` is -1 none, 0 off, 1 on (another negative number is none, a number
above 1 is on). A tone is as a text's.

| call | draws | height given 0 or less | width given 0 or less | returns |
|---|---|---|---|---|
| `text` | text in a role | one line (FIT: one or two; WRAP: all its lines) | the rest of the body from `x` | the height |
| `text_lines` | a page of a text | the rest of the body below `y` | the rest of the body from `x` | its lines |
| `button` | with an icon and a label, the Package Store's action button (`gui_button_row`: the label beside the icon, in Gothic's carved frame or Classic's pill); with one of them alone, Grimoire's button (Gothic's carved one, Classic's pill) | BUTTON_H | what its label and icon need, at least its height | the width |
| `icon_button` | the header's round button (Gothic: carved) | ICON_BUTTON | its height | the width |
| `row` | Settings' row: its card, icon, label (two lines at most) and switch | ROW_H | the rest of the body | the height |
| `chip` | a filter chip; `selected` fills it | CHIP_H | the Package Store's chip: its label in the bold face and 78 px, at least 170 | the width |
| `field` | a text field: icon, text or its hint, the mark that clears it; `flags` 1 draws the text as a star for each character | FIELD_H | the rest of the body | the height |
| `select` | a select: `caption: value` (its first 159 bytes, cut between two characters, before the label rule) and its chevron | SELECT_H | the rest of the body | the height |
| `checkbox` | a checkbox and its label (two lines at most); its rectangle is filled white first | TOUCH_MIN | the rest of the body | the height |
| `card` | a Settings card | ROW_H | the rest of the body | |
| `divider` | the theme's rule at `y`: a line 3 px thick (in Gothic, a thin one 5 px under it) | | the rest of the body | |
| `icon` | an icon by name (`spec.icon`'s names), in a tone | ICON | ICON | |
| `fill` | a rectangle of a gray (0 to 255; another number the nearer end) | nothing is drawn | the rest of the body | |
| `frame` | a box with the theme's corners at RADIUS, filled or outlined `thickness` thick (0 or less: LINE; at most 4,096) | nothing is drawn | the rest of the body | |
| `line` | a line `width` wide (0.1 to 4,096), of a gray | | | |
| `clip` | what is drawn after it is drawn only in its rectangle, cut to the body and to the part of it the host draws on; a later `clip` takes its place, larger or smaller, never past the body | | | |
| `clip_clear` | gives the whole body back | | | |
| `hit` | a hit region, nothing drawn | | | |
| `pager` | Settings' pager | 90 | the rest of the body | the height |

An element given an id of 0 or more is a hit region, the part of its rectangle inside the body:

- a touch that comes down on it gives its id to the events of that contact (a TAP, a PRESS, a drag's: Events);
- in a widget without `kit_event`, a tap on it calls `widget_tap(id)`, as the widget's own `button` and `hit` do;
- in an app, the regions of the last drawing pass are also what the headless runner taps by name (`tap library
  app.hit.N`; the probe `library.app.hits` counts them), and `app_pointer` is still called for every phase.

## Events

A module may export

```text
kit_event(kind, id, a, b, c) -> i32
```

to be told what happens in its body. Both hosts call it with the budget and the stop rules of their other calls:
a call that runs past its instructions or a limit stops the module, as a drawing pass that does (the frame shows
why; the app is closed with a toast). It returns nonzero when its picture changed and should be drawn, 0 when
nothing is to be drawn. A module ignores a kind it does not know, and returns 0.

| kind | name | id | a, b | c |
|---|---|---|---|---|
| 1 | TAP | the hit region under where the contact came down, -1 for none | where it came down (body-local) | 0 touch, 1 pen |
| 2 | PRESS | as TAP | as TAP | as TAP. The contact was held 600 ms within the slop; its lift is no tap |
| 3 | DRAG_START | as TAP | where it came down | as TAP. The contact moved past the slop |
| 4 | DRAG_MOVE | as TAP | the newest sample | how many samples it holds (1 to 256), which `event_points` copies |
| 5 | DRAG_END | as TAP | where it lifted (a cancelled drag: its newest sample) | 1 when the contact was cancelled, else 0 |
| 6 | KEY (apps) | the key's code, as `app_key`'s | its modifiers; 1 pressed, 0 let go | 0 |
| 9 | SETTINGS | 0 | 0, 0 | 0: the reader changed the package's settings, and `setting` answers the new values |

A contact on the body is one of these, by one recogniser for both hosts (`kithost.rs`, `KitPointer`):

- one that lifts within the slop, 5 mm each way of where it came down (59 px at 300 dpi), before 600 ms (Grimoire's
  hold, `HOLD_MS`), is a TAP;
- one held 600 ms within the slop is a PRESS, once, and its lift is nothing;
- one that moves past the slop, a held one too, is a drag: DRAG_START once; then its samples are gathered and handed
  over in DRAG_MOVEs at the pace of its drawing passes (below), the newest 256 when more came; and at its lift a last
  DRAG_MOVE with the samples left, and DRAG_END. A lift past the slop of a contact whose moves were not seen ends a
  drag the same way;
- one the platform cancels (a second finger made a gesture of it, the pen came) or the panel calls palm-sized is no
  tap and no press, and a drag it was ends with DRAG_END marked cancelled (the samples since its last DRAG_MOVE are
  dropped).

A contact begins only in the body, and the recogniser takes every sample of it, in both hosts, also where it leaves
the body: a drag is the same drag in a widget's frame and in an app. So a drag's points (`event_points`) and
DRAG_END's place may lie outside the body, in body coordinates (below 0, or at or past `BODY_W` or `BODY_H`), and a
contact that leaves the body and comes back to lift within the slop before 600 ms is a drag, not a tap. A contact's
moves and its lift come from the source that began it, the pen or a finger.

`event_points(ptr, cap)` writes the points of the DRAG_MOVE being handled into the module's memory at `ptr`,
oldest first, each an x and a y as two i32 (8 bytes, little-endian): as many whole points as `cap` bytes hold, the
event has and the memory takes there (at the memory's end, the bytes of a part of one may be written too). It
returns how many whole points it wrote, and 0 in any call that is not a DRAG_MOVE's.

`key_char(code, mods)` answers the character a key types with those modifiers, as a paired keyboard types into
Grimoire's own keyboards (the US layout, with Shift: letters, digits, punctuation, Space and the keypad), or 0 for
a key that types none (Enter, an arrow, any key with Ctrl, Alt or Meta held). An app that keeps a text of its own
types into it with this.

### How a pass follows input

- Every input event calls the module at once: `kit_event` for each event, and the host's own exports as they always
  were called.
- The drawing pass that input owes is not drawn at once: the host's tick draws it, once for all the input since the
  last pass. While a contact is down, none is drawn while it may still be a tap, so that a tap is one pass, at its
  lift; once it is a press or a drag, at most one every 120 ms (`KIT_PASS_MS`, a selection's drag's pace), refreshed
  fast; after the lift, at once, refreshed as Grimoire's screens are. None is drawn while the panel's last fast
  refresh is still showing. The waits are counted on the app's clock, the same however long the Kindle has been on.
- A fast refresh shows grays as black and white, and a refresh drives only the pixels that change: so once the
  contact has lifted, all that its fast refreshes covered gets one full-depth refresh (a clean of that part alone),
  whatever the lift's pass changed, after a pause: 150 ms after the lift with the pen away, 1.5 s with it near (in a
  frame, the notebook's cleaning of fast refreshes sends it, and draws the frame's part again for it).
- A `kit_event` that returns 0 owes no pass. A call of a host's own export (`app_pointer`, `app_key`,
  `widget_tap`) says nothing of the picture, so it owes one.
- Only the part of the screen the pass changed is refreshed, none when it changed nothing; a widget's pass refreshes
  its body at most. Every pass of an app's is refreshed so, the status bar's minute too.
- A pass that draws the whole body, for whatever reason (a redraw request, Grimoire drawing the notebook again),
  answers what input owed, and a redraw request whose time has come. A widget's pass of part of its body (Grimoire
  drawing a region that holds part of the frame) answers neither: the pass input owes follows, whole. A redraw request
  stands across calls in both hosts, the soonest asked for.
- A flood of input meets these bounds between two screen updates. The main loop reads at most 64 input events
  (`INPUT_BATCH_MAX`) before its tick, and fewer once it has spent 8 ms handling them (`INPUT_BATCH_MS`, checked
  after each event; on a Kindle, where a package's calls take longer, this is the bound that holds). An input event
  calls `kit_event` at most three times (a lift past the slop whose moves were not seen: DRAG_START, DRAG_MOVE and
  DRAG_END) and the host's own export at most once, and the tick calls `kit_event` once more (a PRESS or a
  DRAG_MOVE); the module is drawn once at most. A drag's samples reach it only on the pace's turn and at the lift,
  at most 256 to a DRAG_MOVE, and what the host does for each sample is constant: the recogniser keeps the newest
  256.

### In each host

- **A widget** gets TAP, PRESS and the drag's events in its frame's body, and SETTINGS; no keys, which stay the
  notebook's shortcuts. A module that exports `kit_event` is told of a tap by a TAP and not by `widget_tap`; one that
  does not gets `widget_tap(id)` for a lift within the slop, held or not, as before, and nothing for a drag. The
  title bar, the frame's buttons and its corner are the frame's. A tap's pass is drawn at its lift with the pen near
  or not; a pass the module asks for itself (`request_redraw`) waits for the pen.
- **An app** gets them in its body, KEY and SETTINGS; `app_pointer` is still called as before, for every phase inside
  the body and for the lift wherever it is, and `app_key` for every key, each before `kit_event`. While an app is open the keys are its own: no shortcut of Grimoire's acts but
  `back` and `home`, which leave it ([APPS.md](APPS.md#lifecycle-and-exports)). On a Kindle a keyboard and a
  controller are read only while Settings > Shortcuts is on: an app gets keys while it is on, and none with it off.

## Capabilities

The kit needs no capability of its own. A widget's drawing calls and hit regions need `editor:overlay`, as its own
`rect`, `text`, `button` and `hit` do: without it each does nothing and returns 0. The calls that draw nothing
(`level`, `version`, `metric`, `clock_ms`, `measure_width`, `measure_height`, `list_page`, `event_points`,
`key_char`) need none, nor does `kit_event`. An app may always draw.

## Limits

A kit call is held to the limits of the host it runs in, counted with that host's own calls:

| limit | widget | app |
|---|---|---|
| drawing commands (each drawing call of the kit is one) | 512 in a pass | 4,096 in a call |
| hit regions | 512 in a pass | 512 in a call |
| bytes of one text (each text, label, caption, value and hint the kit reads, and what it measures) | 1,024 | 1,024 |
| bytes of all those texts | 16,384 in a pass | 65,536 in a call |

and to the kit's own:

| limit | |
|---|---|
| an icon's name | 64 bytes; a longer one is not read, and is drawn as a name of no icon is (the grid) |
| what `version` writes | 32 bytes at most |
| the rows `list_page` and `pager` count | 10,000; more is taken as 10,000 |
| the heights `list_page` lays out with | 32,767 px; a larger one is taken as that |
| a frame's outline | 4,096 px thick |
| a line's width | 0.1 to 4,096 px |
| an icon | drawn at most 1,024 px each way; in a larger rectangle it is that size, in the middle |
| a place or a size | any number, as for the hosts' own calls: what is drawn is held to the body, and nothing outside it is drawn |
| the points of a DRAG_MOVE | 256 (`KIT_POINTS_MAX`): the newest, when more were gathered |
| what `event_points` writes | whole points of 8 bytes (`KIT_POINT_BYTES`), at most `cap` bytes and what the memory holds |
| the slop of a tap or a press | 5 mm each way (`TAP_SLOP_MM`) |
| a press | a contact held 600 ms (`HOLD_MS`) |
| the passes input owes while a contact is down | one every 120 ms at most (`KIT_PASS_MS`) |
| the input events between two ticks of the main loop | 64 (`INPUT_BATCH_MAX`), or as many as 8 ms of handling reach (`INPUT_BATCH_MS`); what they call: [How a pass follows input](#how-a-pass-follows-input) |

A length is compared with its limit before the module's memory is read. A call past the command, hit region or
text limit is stopped the way a call that runs past its instruction budget is, and Grimoire's log names the
limit.

## What a call's budget pays for

A kit call is paid from the budget of the host's call it is made in (a widget's 20,000,000, an app's
30,000,000), with the app host's charges, in both hosts ([APPS.md](APPS.md), What a call's budget pays for):

| call | what a call of it takes from the budget |
|---|---|
| every call | 500, whatever it is given and whether or not the package may draw |
| `level`, `metric`, `clock_ms`, `list_page`, `clip`, `clip_clear`, `hit`, `key_char` | nothing more |
| `version` | and 4 for each byte written |
| `measure_width` | and 32 for each byte measured |
| `measure_height` | and 1,024 for each byte: the text broken into lines, each word measured once and a word wider than the line broken between its letters by doubling and halving |
| `text` | and its letters as an app's `text`: for each byte, 32 for each pixel of the role's height up to 64 px, half the square of the height above that, and in the bold and title faces the cube of the height over 128 more; cut short: 16 times the square of its bytes, to 256 bytes; with WRAP: 384 for each byte more; with FIT: 16 times the square of all its bytes more |
| `text_lines` | and its letters as `text`'s, and 384 for each byte |
| `button`, `icon_button`, `row`, `card` | as an app's `button` (three frames), `button` (the icon button: three frames and its icon), `row` and `card`: each frame 1 for every 32 pixels and 2 for each pixel within 32 px of its left and right sides; labels as a text 48 px high fitted to their room (a button's and a row's label on up to two lines: all its bytes in the square); icons by the square of their side |
| `chip`, `field`, `select` | as an app's `chip`, `text_field` and `select`: two frames with round ends, texts on one line, a field's icon as tall as the field, the chevron and the clearing mark of 64 px |
| `checkbox` | its rectangle's pixels by rows, its box and mark (64 px), and its label on up to two lines |
| `divider` | its 10 rows' pixels by rows |
| `icon` | as an app's `icon`: 2 for each pixel of the square of its side (1,024 at most) and 4 for each byte of its name |
| `fill` | 1 for every 32 pixels inside the body |
| `frame` | as an app's `round_rect`: 1 for every 32 pixels, and 2 for each pixel within its radius or its outline's thickness, and 2 px, of its left and right sides |
| `line` | as an app's `line` |
| `pager` | two buttons of three frames and an icon each, and its count |
| `event_points` | and 1 for every 4 bytes it writes (2,048 bytes for 256 points: 512) |

A call whose budget cannot pay a charge ends exactly as a call that ran out of instructions.

The charges were measured in a release build of the tests on a desktop machine. There, the longest pass of kit calls
a budget pays for takes about four times as long as the same budget spent on instructions alone in a widget (4.1)
and about two and a half times in an app (2.7): one FIT text, row or checkbox whose label is 512 one-letter words
that the label rule balances on two lines, in Literata (2.6 and 1.7 in Gothic, about 1.2 and 0.8 in Terminess). A
label or a FIT text of one word wider than its line takes at most 0.3; `measure_height` of 1,024 narrow letters
about the budget's own time for what it pays (1.0, in an app in Literata); `text` with WRAP and `text_lines` at
most 1.0 a pass; a pass of nothing but `chip`, `field` or `pager` calls at most 1.5. An event of nothing but
`event_points` calls that each write 256 points takes about half the time (0.5 in a widget, 0.6 in an app), and one
of `key_char` calls about a tenth. They have not been measured on a Kindle.

## Testing

`rust/tests/kitfix` writes the modules of the kit's tests byte by byte, and the installed packages of
`tests/fixtures/kit/`, `tests/fixtures/kitinput/` and `tests/fixtures/kitpanel/` (`make kit-fixtures`;
`rust/tests/test_kit_fixtures.rs` checks the committed bytes are what it makes). `rust/tests/test_kit.rs` draws every element given no size in both hosts and compares it with
Grimoire's own drawing at the sizes its screens give it, pixel for pixel, in Gothic, Classic and each family of
Settings > Font, and holds each call to its charge and each limit. `tests/scenarios/z_kit_widget.qs` and
`tests/scenarios/z_kit_app.qs` open the fixtures' widget and app in both themes, tap their hit regions by name,
and compare the app's row, given no height, with the row Settings > System draws at the same place.
`rust/tests/test_kit_input.rs` holds the recogniser (a tap, a press, the slop, a drag, a cancel, a flood of
samples), the pace of the passes, the part of the screen a pass changed, and `kit_event`, `event_points` and
`key_char` in both hosts. `tests/scenarios/z_kit_input_app.qs` and `z_kit_input_widget.qs` count the passes and
refreshes of a tap, a long press, a drag, a flood and a drag that leaves the body in an app and in a frame, and their
`_half` and `_wrap` copies the same with the app's clock past 2^31 ms and across its wrap at 2^32;
`z_kit_input_overlap.qs`, `z_kit_input_minute.qs`, `z_kit_input_clean_app.qs` and `z_kit_input_clean_widget.qs`
hold that the panel shows what the screen does after a frame's pass drawn in part, a status bar's minute and the
fast refreshes of a press or a drag; `z_kit_keys.qs` and `z_kit_keys_off.qs` the keys of an open app;
`z_kit_settings.qs` a widget's SETTINGS.
