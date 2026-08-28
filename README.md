<h3 align="center">buoyant</h3>
<h6 align="center">"whatever keeps you afloat"</h6>

a pretty fast linux file explorer with all the features you can ask for (maybe it still lacks some but i'm too lazy to implement them at the time)  

### some notable features

#### nested folder/file creation  
you can use a whole path in the folder creation process  
for example,  
you input `hey/michael/vsauce/here`, then the folder hierarchy will become:  
`📍 youarehere` > `📁 hey` > `📁 vsauce` > `📁 michael` > `📁 here`  

pretty cool, right?

#### multi-pane  
it's basically a mini window manager... yeah  
pane moving and resizing soon!

### installation
clone the repo via `git clone https://github.com/tsukiiru/buoyant`  
build with `cargo build --release`  
the built binary will be in `target/release/`  

aur and nix flake *soon*  

### config
buoyant uses *toml* as the configuration language, more about toml [***here***](https://toml.io/)  
the config file is located in `$HOME/.config/buoyant/buoyant.toml`  

#### [keybinds]

**value**  
type: *String*  
syntax: `[Modifiers] + [Key]`  
example: `Ctrl + Shift + Q` `Alt + P` `ArrowDown`  
*case-insensitive!*  

> [!NOTE]
> [Modifiers] = `[Modifier_1] + [Modifier_2] + [Modifier_3] + ...`  
> Modifier_n options: [`ctrl` `shift` `alt` `super`]

[Key] options: [`a` `b` `c` `d` `e` `f` `g` `h` `j` `k` `l` `m` `n` `o` `p` `q` `r` `t` `u` `v` `w` `y` `z` `arrowup` `arrowdown` `arrowright` `arrowleft` `` ` `` `[` `]` `,` `=` `-` `.` `'` `;` `/` `backspace` `enter` `space` `tab` `delete` `end` `home` `insert` `pagedown` `pageup` `numpadextract` `escape` `printscreen` `pausebreak` `numpad0` `numpad1` `numpad2` `numpad3` `numpad4` `numpad5` `numpad6` `numpad7` `numpad8` `numpad9` `0` `1` `2` `3` `4` `5` `6` `7` `8` `9` *all the function keys*]  

**table**  
|key|default|
|---|---|
|navigate_up|`arrowup`|
|navigate_down|`arrowdown`|
|navigate_forward|`arrowright`|
|navigate_backward|`arrowleft`|
|copy_to_clipboard|`ctrl+c`|
|cut_to_clipboard|`ctrl+x`|
|paste_from_clipboard|`ctrl+v`|
|clear_clipboard|`ctrl+shift+v`|
|delete_selections|`delete`|
|rename_file|`f2`|
|toggle_hidden_view|`ctrl+h`|
|create_file_path|`ctrl+n`|
|create_folder_path|`alt+n`|
|refresh|`ctrl+r`|
|view_info|`f12`|
|search|`/`|
|choice_0|`0`|
|choice_1|`1`|
|choice_2|`2`|
|choice_3|`3`|
|choice_4|`4`|
|choice_5|`5`|
|choice_6|`6`|
|choice_7|`7`|
|choice_8|`8`|
|choice_9|`9`|
|split_vertical|`ctrl + alt + v`|
|split_horizontal|`ctrl + alt + h`|
|close_panel|`ctrl + alt + q`|
|panel_navigate_up|`ctrl + alt + arrowup`|
|panel_navigate_down|`ctrl + alt + arrowdown`|
|panel_navigate_right|`ctrl + alt + arrowright`|
|panel_navigate_left|`ctrl + alt + arrowleft`|

#### [sorting]
**table**
|key|default|type / options|
|--|--|--|
|sorting_by|`"name"`|property / [`name` `accessed` `created` `type` `size` `path`]|
|reversed|`false`|boolean|

#### [view]
**table**
|key|default|type / options|note|
|--|--|--|--|
|explorer|`["name", "size", "type"]`|table<property> / [`name` `size` `type` `created` `accessed`]|order **do** matters|
|metadata|`["name", "type", "size", "accessed", "created"]`|table<property> / [`name` `size` `type` `created` `accessed`]|order **doesn't** matter|
|dark_mode|`false`|boolean||
|format_date|`"%d/%m/%Y, %I:%M:%S %p"`|string|more values in ***[here](https://docs.rs/chrono/latest/chrono/format/strftime/index.html)***|
|info_toast_time|`5000`|uinteger|in milliseconds|
|danger_toast_time|`7000`|uinteger|in milliseconds|
|success_toast_time|`3000`|uinteger|in milliseconds|

#### [clipboard]
**table**
|key|default|type / options|note|
|--|--|--|--|
|behaviour|`"replace"`|clipboard_behaviour / [`replace` `addition`]|configure the behaviour when copying/cutting entries into the clipboard|

### pr && issues
feel free to open prs or issues, the more the merrier :3  

### stuff
icons from [Phosphor Icons](https://phosphoricons.com/)  
