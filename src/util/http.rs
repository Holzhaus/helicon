// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! HTTP client helpers.

use std::time::Duration;

/// Maximum duration for establishing a connection (including the TLS handshake).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum duration a single read from or write to the connection may take.
///
/// This restarts for every read or write, so it detects stalled connections without limiting the
/// total duration of large downloads on slow connections.
const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Create an HTTP agent with the given user agent and timeouts.
///
/// Without timeouts, a connection to a server that stops responding blocks forever (or until the
/// operating system gives up retransmitting, which can take more than 15 minutes).
#[must_use]
pub fn http_agent(user_agent: &str) -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent(user_agent)
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_per_read(Some(IO_TIMEOUT))
        .timeout_per_write(Some(IO_TIMEOUT))
        .build()
        .new_agent()
}
