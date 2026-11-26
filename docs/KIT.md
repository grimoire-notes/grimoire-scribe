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
A package's `min_grimoire_version` stays the install gate.

## Units

The unit is the screen pixel, as for the hosts' own calls: the body's size, a place, a size. Every place is
relative to the body's top left. Sizes come from the theme: `metric` answers the standard sizes Grimoire's
screens use, already at the panel's scale, and an element given a width or a height of 0 or less takes the
standard one (the table of Elements). Each element returns the extent it took (a row its height, a button its
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

## Metrics

Pixels at 300 dpi, the size on every supported panel (Gothic / Classic where they differ). Each is the function
the screen it comes from reads (`globalui.rs`, `ui.rs`), so the two cannot differ.

| id | name | value | from |
|---|---|---|---|
| 1 | TEXT_BODY | 44 | the text size |
| 2 | TEXT_SMALL | 34 | the small size |
| 3 | TEXT_TITLE | 64 | the title size |
| 4 | TEXT_DISPLAY | 112 / 70 | a screen's title (the title size x 1.75 in Gothic's blackletter, x 1.1 in Classic) |
| 5 to 9 | LINE_BODY, LINE_SMALL, LINE_TITLE, LINE_DISPLAY, LINE_MONO | a line of that role: the face's ascent and descent in the font in use | |
| 10 | ROW_H | 118 | Settings' rows |
| 11 | ROW_MIN_H | 90 | Settings' rows when a list is tight |
| 12 | BUTTON_H | 112 | the package manager's actions |
| 13 | CHIP_H | 88 | the home's and the Package Store's filter chips |
| 14 | FIELD_H | 112 | a field is as tall as a button |
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
| 36 | BODY_W | the width of the body the call draws on: a drawing pass's, the last pass's in another call, 0 before the first pass | |
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
  line, and a word wider than the width is broken between its characters. The lines are centred as a block in
  the rectangle, as many as it is high for; when more follow, the last line shown holds the rest, cut short. Given
  no height it takes as many lines as it needs, and `text` returns their height.
- `text_lines` breaks the text the same way and draws, from the top of its rectangle, the lines from `first_line`
  (0 is the first) that its height holds, and returns how many lines the whole text has: a module pages a long
  text so, as the license viewer does. Given no height it takes the rest of the body below `y`.
- A package's text is not reported to Grimoire's translation fit check (a scenario's `GRIMOIRE_CHECK_FIT`).

## Elements

Each is the element Grimoire's own screens draw, in the theme in use. `style` is 0 plain, 1 selected, 2 disabled,
3 primary; another number is plain. `switch` is -1 none, 0 off, 1 on (another negative number is none). A tone is
as a text's.

| call | draws | height given 0 or less | width given 0 or less | returns |
|---|---|---|---|---|
| `text` | text in a role | one line (FIT: one or two; WRAP: all its lines) | the rest of the body from `x` | the height |
| `text_lines` | a page of a text | the rest of the body below `y` | the rest of the body from `x` | its lines |
| `button` | the kit's button (Gothic's carved one, Classic's pill), its label and icon | BUTTON_H | what its label and icon need, at least its height | the width |
| `icon_button` | the header's round button (Gothic: carved) | ICON_BUTTON | its height | the width |
| `row` | Settings' row: its card, icon, label (two lines at most) and switch | ROW_H | the rest of the body | the height |
| `chip` | a filter chip; `selected` fills it | CHIP_H | its label in the bold face and 78 px, at least 170 | the width |
| `field` | a text field: icon, text or its hint, the mark that clears it; `flags` 1 draws the text as a star for each character | FIELD_H | the rest of the body | the height |
| `select` | a select: `caption: value` and its chevron | SELECT_H | the rest of the body | the height |
| `checkbox` | a checkbox and its label (two lines at most); its rectangle is filled white first | TOUCH_MIN | the rest of the body | the height |
| `card` | a Settings card | ROW_H | the rest of the body | |
| `divider` | the theme's rule at `y`: a line 3 px thick (in Gothic, a thin one 5 px under it) | | the rest of the body | |
| `icon` | an icon by name (`spec.icon`'s names), in a tone | ICON | ICON | |
| `fill` | a rectangle of a gray (0 to 255; another number the nearer end) | nothing is drawn | the rest of the body | |
| `frame` | a box with the theme's corners at RADIUS, filled or outlined `thickness` thick (0 or less: LINE; at most 4,096) | nothing is drawn | the rest of the body | |
| `line` | a line `width` wide (0.1 to 4,096), of a gray | | | |
| `clip` | makes the part of the body drawn on smaller, never larger | | | |
| `clip_clear` | gives the whole body back | | | |
| `hit` | a hit region, nothing drawn | | | |
| `pager` | Settings' pager | 90 | the rest of the body | the height |

An element given an id of 0 or more is a hit region, the part of its rectangle inside the body:

- in a widget, a tap on it calls `widget_tap(id)`, as the widget's own `button` and `hit` do;
- in an app, the regions of the last drawing pass are what the headless runner taps by name (`tap library
  app.hit.N`; the probe `library.app.hits` counts them). An app still learns of a touch from `app_pointer`.

## Capabilities

The kit needs no capability of its own. A widget's drawing calls and hit regions need `editor:overlay`, as its own
`rect`, `text`, `button` and `hit` do: without it each does nothing and returns 0. The calls that draw nothing
(`level`, `version`, `metric`, `clock_ms`, `measure_width`, `measure_height`, `list_page`) need none. An app may
always draw.

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
| an icon's name | 64 bytes; a longer one is not read and names no icon |
| what `version` writes | 32 bytes at most |
| the rows `list_page` and `pager` count | 10,000; more is taken as 10,000 |
| the heights `list_page` lays out with | 32,767 px; a larger one is taken as that |
| a frame's outline | 4,096 px thick |
| a line's width | 0.1 to 4,096 px |
| an icon | drawn at most 1,024 px each way; in a larger rectangle it is that size, in the middle |
| a place or a size | any number, as for the hosts' own calls: what is drawn is held to the body, and nothing outside it is drawn |

A length is compared with its limit before the module's memory is read. A call past the command, hit region or
text limit is stopped the way a call that runs past its instruction budget is, and Grimoire's log names the
limit.

## What a call's budget pays for

A kit call is paid from the budget of the host's call it is made in (a widget's 20,000,000, an app's
30,000,000), with the app host's charges, in both hosts ([APPS.md](APPS.md), What a call's budget pays for):

| call | what a call of it takes from the budget |
|---|---|
| every call | 500, whatever it is given and whether or not the package may draw |
| `level`, `metric`, `clock_ms`, `list_page`, `clip`, `clip_clear`, `hit` | nothing more |
| `version` | and 4 for each byte written |
| `measure_width` | and 32 for each byte measured |
| `measure_height` | and 384 for each byte: the text broken into lines, each word measured once and a word wider than the line broken between its letters by doubling and halving |
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

A call whose budget cannot pay a charge ends exactly as a call that ran out of instructions.

## Testing

`rust/tests/kitfix` writes the modules of the kit's tests byte by byte, and the installed packages of
`tests/fixtures/kit/` (`make kit-fixtures`; `rust/tests/test_kit_fixtures.rs` checks the committed bytes are what
it makes). `rust/tests/test_kit.rs` draws every element given no size in both hosts and compares it with
Grimoire's own drawing at the sizes its screens give it, pixel for pixel, in Gothic, Classic and each family of
Settings > Font, and holds each call to its charge and each limit. `tests/scenarios/z_kit_widget.qs` and
`tests/scenarios/z_kit_app.qs` open the fixtures' widget and app in both themes, tap their hit regions by name,
and compare the app's row, given no height, with the row Settings > System draws at the same place.
