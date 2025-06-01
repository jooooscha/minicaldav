// minicaldav: Small and easy CalDAV client.
// Copyright (C) 2022 Florian Loers
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! minicaldav
//!
//! minicaldav is a caldav client and basic ical parser with as little dependencies as possible (but practical).
//!
//! minicaldav should be
//! - Simple: Few dependencies, no async, the code is simple
//!
//! minicaldav is work in progress
//! - The code is not feature complete
//! - The code is not correct
//!
//! # Quick Start
//!
//! ```rust,no_run
//! let agent = ureq::Agent::new();
//! let url = url::Url::parse("http://mycaldav.com/").unwrap();
//! let username = "foo";
//! let password = "s3cret!";
//! let credentials = minicaldav::Credentials::Basic(username.into(), password.into());
//! let calendars = minicaldav::get_calendars(agent.clone(), &credentials, &url).unwrap();
//! for calendar in calendars {
//!     println!("{:?}", calendar);
//!     let credentials = minicaldav::Credentials::Basic(username.into(), password.into());
//!     let (events, errors) = minicaldav::get_events(agent.clone(), &credentials, &calendar).unwrap();
//!     for event in events {
//!         println!("{:?}", event);
//!     }
//!     for error in errors {
//!         println!("Error: {:?}", error);
//!     }
//! }
//! ```

#[cfg(any(feature = "caldav", feature = "ical"))]
#[macro_use]
extern crate log;

pub mod errors;

#[cfg(feature = "caldav")]
mod api;
#[cfg(feature = "caldav")]
pub mod caldav;
#[cfg(feature = "caldav")]
pub use api::*;

#[cfg(feature = "caldav")]
mod xml_templates;

#[cfg(feature = "ical")]
pub mod ical;


mod credentials;
