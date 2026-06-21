## buoyant - (arguably) the best file explorer on linux
### made by truly yours, tsuki, the worst coder on the planet

<h6 align="center">meow meow meow meow meow meow meow meow meow meow meow meow</h6>

### backstory
i know you dont care about any of this but im bored, you can scroll by if you want, i dont care either,... hmph!  
anyway, i dont like how mainstream file explorers like nautilus or dolphin are dependent on their mother (GTK and KDE), no hate btw, but just like nautilus just,,, lacks alot of features, and there is this "no theming my app" thing which (honestly?) sucks. or dolphin, the ui looks absolutely horrible, and like you need a brain to go f*ck around with all the theming, whose documentation and codebase is even more unc than me  
im currently working on buoyant just to replace these two, and (hopefully) use it as my mainstream file explorer so i wont fucking crash out on the terrible experience i already have and will have with the currently functional ones.  
dont ask why the name is buoyant though, i think its from bee swarm simulator, yk, the buoyant bee?  

by the way, i made it for me, i dont care about you, so like, if it breaks anything its not my fault, because you chose this instead of anything else, please still open an issue though, ill try to fix it asap  

### state
yup, its actively being developed (in my free time) 🫰, by [me](https://github.com/tsukiiru)!!  
heavily in development state and experimental, not like theres gonna be many breaking changes *(subtle foreshadowing)* though  
<sub>note: do not use this right now please, i cant guarantee it wont delete your entire system</sub>

*[pretend theres a preview picture here with blurred background and a giant stylish logo in the middle]*

### fun stuff
- amazing configuration file in toml language, from a to z (only keybinds is supported currently now tho)
- keybinds-heavy
- yeah thats it for now...

#### creating file or folder
you can input a whole long path into it, for example:  
`hey/michael/vsauce/here`  
lets say im creating a new folder then in the current directory, we will have this:  
`📁 youarehere` > `📁 hey` > `📁 michael` > `📁 vsauce` > `📁 here`  
so cool right?

#### visual mode?
its something in neovim  
basically it lets you select multiple files when its on  

#### delete
no trash bin, no return ok?

### config 
buoyant uses toml as the configuration language, please search it up if you want to know about the syntax!  
the config file is located at `~/.config/buoyant/buoyant.toml`, its not created automatically so go make it yourself  

#### [view]
for displaying various information  

**key: explorer** - the columns on the explorer  
type: table  
default: `["name"]`  
available values: name, last_accessed, created, filesize, filetype  
note: order does preserve from left to right  

**key: metadata** - what to show in the metadata panel  
type: table  
default: `["name", "last_accessed", "created", "filetype", "filesize"]`  
available values: name, last_accessed, created, filesize, filetype  
note: order preserves (left to right) -> (top to bottom)  

#### [keybinds]
**Value syntax:** `"[MODIFIERS] + [KEY]"`
for example: `"Ctrl + Shift + H"`
    or like: `"Alt + P"`

```toml
[keybinds]
navigate_up = "arrowup"
navigate_down = "j"
# i didnt lie when i said as many mods as you like
cut_to_clipboard = "ctrl + ctrl + ctrl + ctrl + ctrl + x"
```

> [!NOTE]
> there can only be **1** KEY, can be as many modifiers as you want,  
> KEY, MODIFIERS, and each MODIFIER is separated by `+`  
> oh and they have to be **String** too btw *(coated in double quotes "")*

**Keys List:** a, b, c, d, e, f, g, h, j, k, l, m, n, o, p, q, r, t, u, v, w, y, z, arrowup, arrowdown, arrowright, arrowleft, `, [, ], ,, =, -, ., ', ;, /, backspace, enter, space, tab, delete, end, home, insert, pagedown, pageup, numpadextract, escape, printscreen,  pausebreak, numpad0, numpad1, numpad2, numpad3, numpad4, numpad5, numpad6, numpad7, numpad8, numpad9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11, f12,...to f35  

**Modifiers List:** ctrl, shift, alt, super  

they arent case-sensitive, put whatever you want in, just make sure they look correctly  

**Options:**
|Key|Default|Description|
|---|---|---|
|navigate_up|arrowup|for navigating up|
|navigate_down|arrowdown|for navigating down|
|navigate_forward|arrowright|enter selected file|
|navigate_backward|arrowleft|go to parent directory|
|copy_to_clipboard|ctrl+c|.....|
|cut_to_clipboard|ctrl+x|....|
|paste_from_clipboard|ctrl+v|....|
|delete_selections|delete|erase the selected file(s) from existence|
|rename_file|f2|....|
|toggle_hidden_view|ctrl+h|lets you see or not see dotfiles|
|create_file_path|ctrl+n|create file from the current directory|
|create_folder_path|alt+n|create folder from the current directory|
|toggle_visual_mode|v|toggle visual mode (for selecting files with keybinds)|
|refresh|ctrl+r|refresh explorer with the config|

#### [sorting]
for sorting items on the explorer  

**key: sorting_by**  
default: `"name"`  
type: string  
available values: name, accessed, created, type, size   

**key: reversed**
default: `false`  
type: boolean  

#### [misc]
for misc things that cant be put anywhere else  

**key: format_time** - how to format the metadata time  
default: `"%d/%m/%Y, %I:%M:%S %p"`  
type: string  
it follows [this](https://docs.rs/chrono/latest/chrono/format/strftime/index.html)  

**key: accurate_filesize** - use a *really advanced* algorithm to get a more accurate file size in the metadata  
default: `false`  
type: boolean  
note: this makes it really laggy, use at your own risk (unless you got a real good computer)  

### contribute
woaw  
can someone please, i dont wanna write the readme  

<a href="https://github.com/iced-rs/iced">
    <img src="https://gist.githubusercontent.com/hecrj/ad7ecd38f6e47ff3688a38c79fd108f0/raw/74384875ecbad02ae2a926425e9bcafd0695bade/color.svg" width="130px">
</a>
