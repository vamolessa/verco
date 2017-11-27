# verco
A simple Git/Hg version control client based on keyboard shortcuts

## Install

First of all, install rust into your system using [rustup](https://www.rustup.rs/).
Now you can install verco using Cargo (Rust's package manager).

Now we can clone this repo and install verco.

Open a terminal and run:

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
