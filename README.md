# Hypr - Bucket 🪣

>
> Lightweight and customizable application launcher for hyprland

[![License](https://img.shields.io/badge/License-GPLv3-blue)](https://www.gnu.org/licenses/gpl-3.0)

## ❗️❗️❗️**Recommended** ❗️❗️❗️

> If you want a better blur effect add this to your windowrules/layerrules in hyprland!

```layerrule = blur, hyprbucket```

```layerrule = ignorealpha 0.5, hyprbucket```

You can style the launcher yourself by adding your own css in:
```~/.config/hyprbucket/style.css```

## Disclaimer

Hypr-Bucket is **heavily inspired by**, and includes portions of code from, the excellent  
[walker](https://github.com/abenz1267/walker) project by @abenz1267.  
While walker offers a feature-rich and highly customizable launcher, Hypr-Bucket focuses on being **lightweight**, **optimized**.

## Installation

### Arch Linux (AUR)

```bash
yay -S hypr-bucket
```

### Build from Source

> Dependencies:

- Rust 1.70+
- GTK4 development libraries
- GTK4 Layer Shell

```bash
git clone https://github.com/Time-0N/hypr-bucket
cd hypr-bucket
cargo build --release
sudo install -Dm755 target/release/hbucket /usr/local/bin/hbucket
mkdir -p ~/.config/hyprbucket
install -Dm644 resources/default.css ~/.config/hyprbucket/default.css
```

## Technicalities

Pinned apps are stored in `~/.config/hyprbucket/config.toml`. Desktop entry cache is stored in `~/.cache/hyprbucket/desktop_entries.json`.
