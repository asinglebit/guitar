<div align="center">
  <pre>
    </br>
                              :GG~        .?Y.                                
    ....        ..      ..   .....      . ^BG: ..       .....                 
 .7555YY7JP^   ~PJ     ~PJ  ?YY5PP~    7YY5BGYYYYJ.   J555YY557.              
.5B?.  :JBB~   !#5     !#5  ...PB~     ...^BG:....    ~:.   .7#5           :^^
7#5     .GB~   !B5     !B5     PB~        :BG.        .~7??J?JBG:      .~JPPPY
?#Y      PB~   !B5     !B5     PB~        :BG.       7GP7~^^^!BG:     ~5GY!:. 
^GB~    7BB~   ^BG.   .YB5     5#7        :BB:       P#!     JBG:    ^GG7     
 ^5G5JJYJPB~    JBP???YYB5     ^5GYJJ?.    7GPJ???.  ~PGJ77?5J5B!    JG5      
   .^~^..GB:     :~!!~. ^^       :~~~~      .^~~~~    .^!!!~. .^:    JG5      
 .?!^^^!5G7                                                          YB5      
 .!?JJJ?!:                                                           75?      
  </br>
Beautiful and cozy terminal-based Git client
 for fun, productivity, and pure Rust joy.  
  </pre>
</div>

![2](https://github.com/user-attachments/assets/177dbf13-b9ad-480e-a1be-71a333454a44)

### Disclaimer

I work on `guita╭` in my spare time, and give priority to the features I need in my day-to-day life. Use it with caution, and feel free to report issues or even better - contribute improvements! Im a lazy dude, and since this is a hobby project I just put unwrap() everywhere, such is life. `guita╭` is a tool I sometimes work on to make my day-to-day life as a developer easier.

### Motivation

I needed a git client that would make it easy for me to understand where I am topologically at any given point in time. I also want it to be terminal based and cross-platform. I needed it to be fast. I also wanted to learn rust. So this is the project i picked to meet all of these goals at the same time.

### Features

- **Beautiful graph rendering** – Visualize commit history clearly.  
- **Immediate jumps** – Move through history without waiting.  
- **Pure TUI experience** – Ratatui based rendering.
- **Auth** – Currently simply attaches to the running ssh agent.  
- **Built-in diff viewer** – Inspect changes without leaving the terminal, however its very rudimentary.  
- **Tag management** – Create, view or remove tags.  
- **Stash management** – Create, view or remove stashes.  
- **Cherrypicking** – Happy path only for now.  
- **Opinionated** – Fetches prune branches and pull tags. Pushes are always hard and push local tags.
- **Keymap** – Keymap is completely customizable and is serilazied into `~/.config/guitar` folder (depending on your OS).
- **Layout** – Somewhat primitive, also serilazied into `~/.config/guitar` folder (depending on your OS).
- **Terminal-friendly colors** – Easy on the eyes for long coding sessions with three builtin themes. 

### Planned features

- **Rebasing** – Ability to rebase.  
- **Merge** – Ability to merge.  
- **Conflicts** – Issue an alert with conflicting files to resolve externally.  

### Maybe I get to it someday features

- **Auth** – Comprehensive auth management.
- **Keymaps** – In-app keymap configuration.
- **Themes** – Customizable themes.
- **Recent repos** – List of most recent repositories.
- **Heatmap** – Render a github-style heatmap of the repository.

### Keyboard mappings

<div align="center">
<pre>
╭─────────────────────────────────────────────────────────────────────╮
│ [_]   [_][_][_][_] [_][_][_][_] [_][_][_][_] [_][_][_] [_][_][_][_] │
│                                                                     │
│ [_][_][_][_][_][_][_][_][_][_][_][_][_][___] [_][_][_] [_][_][_][_] │
│ [__][_][_][_][_][_][_][_][_][_][_][_][_][_ │ [_][_][_] [_][_][_][ | │
│ [___][_][_][_][_][_][_][_][_][_][_][_][_][_│           [_][_][_][_| │
│ [_][_][_][_][_][_][_][_][_][_][_][_][______]    [_]    [_][_][_][ | │
│ [__][_][__][_____________________][__][_][_] [_][_][_] [____][_][_| │
╰─────────────────────────────────────────────────────────────────────╯

Select                                                            Enter
Next Pane                                                           Tab
Previous Pane                                               Shift + Tab
Page Up                                                          Pageup
Page Down                                                      Pagedown
Scroll Up                                                            Up
Scroll Down                                                        Down
Scroll Up Half                                               Shift + Up
Scroll Down Half                                           Shift + Down
Scroll Up Branch                                              Ctrl + Up
Scroll Down Branch                                          Ctrl + Down
Scroll Up Commit                                               Alt + Up
Scroll Down Commit                                           Alt + Down
Go To Beginning                                                    Home
Go To End                                                           End
Jump                                                                  j
Solo Branch                                                           o
Drop                                                                  y
Pop                                                                   t
Stash                                                                 e
Grep                                                                  g
Fetch                                                                 f
Checkout                                                              c
Hard Reset                                                            h
Mixed Reset                                                           m
Unstage All                                                           u
Stage All                                                             s
Commit                                                                a
Push                                                                  p
Create A New Branch                                                   b
Delete A Branch                                                       d
Tag                                                                   /
Untag                                                                 ?
Cherrypick                                                            ]
Go Back                                                             Esc
Reload                                                                r
Minimize                                                              .
Toggle Shas                                                           w
Toggle Branches                                                       `
Toggle Tags                                                           3
Toggle Stashes                                                        4
Toggle Status                                                         2
Toggle Inspector                                                      1
Toggle Settings                                                      F1
Exit                                                           Ctrl + c
</pre>
</div>

### Releases

Please check the releases for the latest versions: https://github.com/asinglebit/guitar/releases

### Build yourself

Clone the repo and build with Cargo:

```bash
git clone https://github.com/asinglebit/guitar.git
cd guitar
cargo build --release
```

Your binary path after a successfull build:

```bash
guitar/target/release/guitar
```

Copy it wherever you want and run with a path to repo you wish to inspect

```bash
guitar ../path/to/your/repo
```

### Screenshots
![1](https://github.com/user-attachments/assets/37df457b-bbf4-4d51-a965-c300b426cb62)
![6](https://github.com/user-attachments/assets/2ed8f14e-193b-4815-b37e-283bd129787f)
![5](https://github.com/user-attachments/assets/15e4630f-a141-4724-9d35-1b8601006598)
![4](https://github.com/user-attachments/assets/10389ec5-6780-4bcb-85dc-67f9e012ed63)
![3](https://github.com/user-attachments/assets/a408af0c-ef85-4692-914b-81562d3873e4)
