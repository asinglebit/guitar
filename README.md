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
terminal based cross-platform git client 
made with ♡ by @asinglebit
  </pre>
</div>

![2](https://github.com/user-attachments/assets/177dbf13-b9ad-480e-a1be-71a333454a44)

### Demo

Heres a recording of me going through the features of v0.1.12
[https://www.youtube.com/watch?v=oERA8MYlHjQ](https://www.youtube.com/watch?v=oERA8MYlHjQ)

### Disclaimer

I work on `guita╭` in my spare time and prioritize the features I need in my day-to-day life. Use it with caution, and feel free to report issues—or even better, contribute improvements! I’m a lazy dude, and since this is a hobby project, I just put unwrap() everywhere - such is life. I’m slowly working on improving every aspect of it.

### Motivation

I needed a git client that would make it easy for me to understand where I am topologically at any given point in time. I also wanted it to be terminal based and cross-platform. I needed it to be fast. I also wanted to learn rust. So this is the project i picked to meet all of these goals at the same time.

### Features

- Fast graph rendering of the entire repository, with isntant random access (hundreds of thousands of commits, spanning decades)
- Vim-like navigation
- Reload the client manually using the shortcut when needed. Doesn't watch the directory
- List recent repositories for ease of access
- Automatically attaches to the ssh agent. Just make sure it runs.
- Diff inspector
- Tag management
- Stash management
- Branch management
- Commit metadata inspector
- Staging, unstaging, commiting, fetching and pushing
- Chrerypicking (happy path only for now)
- Keymap is completely customizable and is serilazied into `~/.config/guitar` folder (depending on your OS).
- Layout, also serilazied into `~/.config/guitar` folder (depending on your OS).
- Zen mode, for one pane based interaction
- Comes with three color themes
- Rendering using ratatui

### Roadmap

Planned features: jujutsu integration, git worktrees, conflict mode, merging, rebasing and more...

Follow the link to get an idea of the roadmap and see what currently is being worked on:

https://github.com/users/asinglebit/projects/1/views/1

### Default keyboard mappings

You will probably need to change the configs to your taste, for optimal experience, especially on MacOS due to Option/Command keys. I will version configs properly in the future. Dangerous actions, like resetting and force pushing are now behind `Action Mode` key. You need to enter the `Action Mode` first, and then next shortcut you commit will be picked from the `Action Mode` key pool.

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

shortcuts / normal mode:                                                                

Widen Scope                                                           h
Narrow Scope                                                          l
Widen Scope                                                        Left
Narrow Scope                                                      Right
Select                                                            Enter
Back                                                                Esc
Focus Previous Pane                                            Ctrl + p
Focus Next Pane                                                Ctrl + n
Focus Next Pane                                                     Tab
Focus Previous Pane                                     Shift + BackTab
Scroll Down                                                           j
Scroll Up                                                             k
Scroll Down                                                        Down
Scroll Up                                                            Up
Scroll Down Half                                         Ctrl + Alt + d
Scroll Up Half                                           Ctrl + Alt + u
Scroll Half Page Down                                          Ctrl + d
Scroll Half Page Up                                            Ctrl + u
Scroll Page Up                                                   PageUp
Scroll Page Down                                               PageDown
Go To Beginning                                                       g
Go To End                                                     Shift + G
Go To Beginning                                                    Home
Go To End                                                           End
Find                                                                  /
Scroll Up Branch                                              Shift + {
Scroll Down Branch                                            Shift + }
Scroll Up Commit                                                      [
Scroll Down Commit                                                    ]
Toggle Zen Mode                                                       z
Toggle Hunk Mode                                                      m
Toggle Branches                                                       1
Toggle Tags                                                           2
Toggle Stashes                                                        3
Toggle Status                                                         4
Toggle Inspector                                                      5
Toggle Shas                                                           6
Toggle Help                                                           ?
Action Mode                                                    Ctrl + a
Minimize                                                              .
Reload                                                                r
Exit                                                                  q
Stage                                                                 s
Unstage                                                               u
Commit                                                                c
Fetch All                                                             f
Create Branch                                                         b
Tag                                                                   t
Toggle Branch                                                 Shift + T
Solo Branch                                                       Space
                                                                       
additional shortcuts / action mode (prefix: Ctrl + a):                                    
                                                                       
Drop                                                                  x
Pop                                                                   p
Stash                                                         Shift + S
Checkout                                                              o
Hard Reset                                                    Shift + H
Mixed Reset                                                   Shift + M
Force Push                                                    Shift + P
Delete Branch                                                 Shift + D
Untag                                                         Shift + U
Cherrypick                                                            y

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

Or alternatively, alias the executable and then call it from a repo folder directly.

Running it from a non repo folder will crash the process. I will fix it sometime later...

### Screenshots

![1](https://github.com/user-attachments/assets/37df457b-bbf4-4d51-a965-c300b426cb62)
![1](https://github.com/user-attachments/assets/b09f73e7-5bda-4ecd-ba7c-3bba85395e37)
![6](https://github.com/user-attachments/assets/2ed8f14e-193b-4815-b37e-283bd129787f)
![5](https://github.com/user-attachments/assets/15e4630f-a141-4724-9d35-1b8601006598)
![4](https://github.com/user-attachments/assets/10389ec5-6780-4bcb-85dc-67f9e012ed63)
![3](https://github.com/user-attachments/assets/a408af0c-ef85-4692-914b-81562d3873e4)
