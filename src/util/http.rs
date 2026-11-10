// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! HTTP client helpers.

use crate::config::LookupConfig;
use std::time::Duration;

/// Convert a timeout in seconds from the configuration, where 0 means no timeout.
fn timeout_from_secs(secs: u64) -> Option<Duration> {
    (secs > 0).then(|| Duration::from_secs(secs))
}

/// Create an HTTP agent with the given user agent and the timeouts from the configuration.
///
/// Without timeouts, a connection to a server that stops responding blocks forever (or until the
/// operating system gives up retransmitting, which can take more than 15 minutes).
#[must_use]
pub fn http_agent(user_agent: &str, config: &LookupConfig) -> ureq::Agent {
    let io_timeout = timeout_from_secs(config.io_timeout);
    ureq::Agent::config_builder()
        .user_agent(user_agent)
        .timeout_connect(timeout_from_secs(config.connect_timeout))
        .timeout_per_read(io_timeout)
        .timeout_per_write(io_timeout)
        .build()
        .new_agent()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_timeout_from_secs() {
        assert_eq!(timeout_from_secs(0), None);
        assert_eq!(timeout_from_secs(5), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_stalled_connection_times_out() {
        // Accept the connection, but never respond.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let _handle = std::thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_secs(10));
        });

        let config = LookupConfig {
            io_timeout: 1,
            ..crate::Config::default().lookup
        };
        let started = Instant::now();
        let err = http_agent("helicon-test", &config)
            .get(&url)
            .call()
            .unwrap_err();
        assert!(matches!(err, ureq::Error::Timeout(_)), "{err:?}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
