# ddc-brightness-ctl

This is a small program offering an `xbacklight`-style interface to control the brightness of monitors with support for [DDC](https://en.wikipedia.org/wiki/Display_Data_Channel) luminance control.

```
Usage: ddc-brightness-ctl [-d|--display=NUM] [-l|--list] [--inc=NUM] [--dec=NUM] [--set=NUM]

Options:
  -d, --display: optionally specify which display to change
                 default operates on all displays
  -l,    --list: list all detected displays and metadata
          --set: set brightness to NUM percent
          --inc: increase brightness by NUM percent
          --dec: decrease brightness by NUM percent
```

## Installation

Pick your poison:
```shell
cargo install --locked ddc-brightness-ctl
cargo install --locked --git https://github.com/tritoke/ddc-brightness-ctl.git
# if you have have the repo cloned:
cargo install --path . --locked
```

Note: a manual page is also provided at `ddc-brightness-ctl.1`, this can be installed with:
```
mkdir -p ~/.local/share/man/man1
curl https://raw.githubusercontent.com/tritoke/ddc-brightness-ctl/refs/heads/main/ddc-brightness-ctl.1 -o ~/.local/share/man/man1/ddc-brightness-ctl.1
```
