# Helicon [![License][license-badge]][license] [![Build Status][build-badge]][build]

Helicon is a command-line tool to tag and organize your music based on metadata
from [MusicBrainz][musicbrainz], written in Rust.

It's heavily influenced by [*beets*][beets], but does some things differently.
One major difference is that unlike *beets* it does not maintain a database of
imported releases in order to achieve better performance with large libraries.

**Note:** This crate is still in early stages of development and not ready for
production use.

## License

This software is [licensed][license] under the terms of the [Mozilla Public License
2.0](https://www.mozilla.org/en-US/MPL/2.0/). Please also have a look at the
[license FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/).

[beets]: https://beets.io/
[musicbrainz]: https://musicbrainz.org/
[license]: https://github.com/Holzhaus/helicon/blob/main/COPYING
[license-badge]: https://img.shields.io/github/license/Holzhaus/helicon
[build]: https://github.com/Holzhaus/helicon/actions?query=branch%3Amain
[build-badge]: https://img.shields.io/github/workflow/status/Holzhaus/helicon/Build
