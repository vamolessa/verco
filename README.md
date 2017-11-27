# verco
A simple Git/Hg version control client based on keyboard shortcuts

## Platforms

This project uses Cargo and pure Rust stable so Windows, Mac and Linux should work.

It depends on:
- [rustyline](https://github.com/kkawakam/rustyline)
- [termion windows fork](https://github.com/mcgoo/termion)
  - `windows` branch
  - watch the windows port [issue](https://github.com/ticki/termion/issues/103)

## Install

First of all, install rust into your system using [rustup](https://www.rustup.rs/).

Once it's installed, you can proceed to install verco using Cargo (Rust's package manager).
Open a terminal and run these commands to clone and install verco:

```
git clone https://github.com/matheuslessarodrigues/verco.git
cd verco
cargo install
```

Once you close and open again your terminal, you'll be able to use `verco` in whichever directory you need.
You can even delete that `verco` folder if you please.

## Usage

Open a terminal from your repository folder and type in `verco`.
It will launch verco and you can begin to use it.

Use your keyboard to perform git/hg actions (also, press `h` for help).

## Keymap

```
h               help

s               status
l               log

c               commit
shift+r         revert
u               update/checkout
m               merge

f               fetch
p               pull
shift+p         push

shift+t         new tag
b               list branches
shift+b         new branch
```

## Screenshots

![verco video example](https://raw.githubusercontent.com/matheuslessarodrigues/verco/master/images/example.mp4)

![help screen in verco](images/help.png)

![commit screen in verco](images/commit.png)

![log screen in verco](images/log.png)
