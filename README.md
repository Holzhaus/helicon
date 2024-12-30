# Helicon [![License][license-badge]][license] [![Build Status][build-badge]][build] [![pre-commit.ci status][pre-commit-badge]][pre-commit]

Helicon is a command-line tool to tag and organize your music based on metadata
from [MusicBrainz][musicbrainz], written in Rust.

**Note:** This crate is still in early stages of development and probably not
ready for production use. Don't forget to make backups!

![Helicon in Action](assets/helicon.gif)

## Installation

Just clone the repository and install the crate as usual:

```bash
$ git clone https://github.com/Holzhaus/helicon.git
$ cd helicon
$ cargo install --path .
```

Don't forget to make sure that your `$PATH` includes `$HOME/cargo/bin`.

## Usage

By default, Helicon will import files into `~/Music`. If you want to use a
different directory, create a config file:

```bash
$ mkdir -p ~/.config/helicon
$ printf '[paths]\nlibrary_path = "/path/to/music/library/"\n' > ~/.config/helicon/config.toml
```

To check if your configuration was recognized correctly, use the `config` command:

```bash
$ helicon config
```

If all looks good, you can import your first album by running:

```bash
$ helicon import ./path/to/some/music/to/import
```

Check the output of the `--help` flag for details.

## Q & A

### How does Helicon compare to Beets?

Helicon is heavily influenced by [*beets*][beets], but does some things
differently. Apart from being written in Rust instead of Python, one major
difference is Helicon does not maintain a database of imported releases in
order to achieve better performance with large libraries on low-end hardware.

### What are the Design Goals and Non-Goals?

Helicon strives to be performant, efficient to use and should support storing
as much information from MusicBrainz as possible.

Helicon should be relatively self-contained (i.e., not rely on external
command-line applications to be installed) and there are also no plans for a
third-party plugin architecture.

### Why the name "Helicon"?

!["Apollo and the Muses on Mount Helicon" (Claude Lorrain, 1680)](assets/mount-helicon.jpg)

*Helicon* is named after Mount Helicon (Ἑλικών in Ancient Greek), which is -- according to Greek mythology --  the home of the Muses, the nine goddesses of knowledge and the arts, including music.


## License

This software is [licensed][license] under the terms of the [Mozilla Public License
2.0](https://www.mozilla.org/en-US/MPL/2.0/). Please also have a look at the
[license FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/).

[beets]: https://beets.io/
[musicbrainz]: https://musicbrainz.org/
[license]: https://github.com/Holzhaus/helicon/blob/main/COPYING
[license-badge]: https://img.shields.io/github/license/Holzhaus/helicon
[build]: https://github.com/Holzhaus/helicon/actions?query=branch%3Amain
[build-badge]: https://img.shields.io/github/actions/workflow/status/Holzhaus/helicon/build.yml?branch=main
[pre-commit]: https://results.pre-commit.ci/latest/github/Holzhaus/helicon/main
[pre-commit-badge]: https://results.pre-commit.ci/badge/github/Holzhaus/helicon/main.svg
