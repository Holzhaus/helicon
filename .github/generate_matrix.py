#!/usr/bin/env python3
# Copyright (c) 2024 Jan Holthuis <jan.holthuis@rub.de>
#
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
# the MPL was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

import argparse
import tomllib
import json
import itertools


def generate_matrix(features: list[str]):
    for i in range(len(features) + 1):
        yield from itertools.combinations(iter(features), i)


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("cargo_file", type=argparse.FileType("rb"))
    parser.add_argument("output_file", type=argparse.FileType("w"))
    args = parser.parse_args(argv)

    cargo_toml = tomllib.load(args.cargo_file)
    features = [
        feature for feature in cargo_toml["features"].keys() if feature != "default"
    ]
    matrix = {
        "include": [
            {"features": "{}".format(",".join(f))}
            for f in generate_matrix(features=features)
        ]
    }

    args.output_file.write("matrix={}".format(json.dumps(matrix)))


if __name__ == "__main__":
    main()
