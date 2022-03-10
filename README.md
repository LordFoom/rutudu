# rutudu
The simple, tui rust to do list. You can say it roo-too-doo or rah-tah-dah, or even rah-too-doo!

Wanted a TUI based todo list with hierarchies that doesn't just cat to the console,
but allows navigation,collapsing, etc. Couldn't find one so here we are.

I  want to be able to go fast, so the idea is to have a vim-ish interface where single key strokes,
in the right mode, perform activities quickly.

Work in progress but you can:

* Make simple list, with entry text
* Sub lists.


You cannot edit, just yet.

You can cross-out and and uncross-out. Move items up and down - sibling-list and hierarchy.

Will auto open a new list with today's date, creating if necessary, if started with no arguments. Otherwise, will
open/create passed in list name.

There is no undo. Sweaty palms but steady hands, my friend.

Busy adding some time tracking integration. The rust of clocks is timesheets.

### You can add items!

* a to *a*dd a new sibling
* Ctrl+a to add a sub item
* Alt+a to add new parent
* Shift + A to add new root item
* __It is CTRL+N  to e(N)ter on the add item screen__
    * That's because enter is for newlines when creating items.
  * __As of recently, also Alt+Enter!! :D__
* Seriously, if somebody can teach me how to capture CTRL+enter, I'd be so grateful
  * Can't seem to modify enter on the terminal?
  * HOORAH! Managed to get Alt+Enter working
  * CTRL+Enter would be nice, though

### Manipulate items in the list
* x to (un)cross out item
* u move item *u*p (increase its rank among its siblings)
* d move item *d*own (decrease its rank among its siblings)
* i or \> move item *i*n (become the child of preceding sibling)
* o or < move item *o*ut (become the sibling of its parent)
* delete or backspace to delete an item - but NOT its children
* ctrl+e to *e*rase an item (delete it AND its children)
* alt+m to *m*ark an item (orange)

### Persistence
* s to save to sqlite file 
* shift+S to 'save as...'
* o to load (up/down to select, press right/enter/ctrl+n to open)
* It's a sqlite file so now you have the data in a db, maybe that's cool for you.

###If built/running with 'clockrust' feature
* ctrl+t run "clock-in" or "clock-out" command to store clock_rust_tasks table in the sqlite db file

## Installation

Check out of git and build --  cargo build

If you want the time tracking (still alpha and evolving): cargo build --features clockrust

I like having lots of short lists and they're stored for posterity in sqlite. Maybe in future, some kind of tool
to aggregate lists? 

Will integrate with my clockrust, my nascent timetracking project. In time. In time.

### ROADMAP

* edit

<img src="./item_manipulation.gif" title="item manipulation" />
<img src="./example_pic.png" title="Looks like this" >
<img src="./grey_crossed_out.png" title="Now with soothing crossed out items" >
<img src="./open_file.png" title="Opening files" >
<img src="./rutud_1.gif" title="The cursor works...in the forward direction" width="1046" height="555">
<img src="./hierarchies.png" title="sub-lists"/>