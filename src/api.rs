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

//! Main api of minicaldav.

use crate::caldav;
use crate::ical;
use ureq::Agent;
use url::Url;

/// Get all calendars from the given CalDAV endpoint.
pub fn get_calendars(
    agent: Agent,
    username: &str,
    password: &str,
    base_url: &Url,
) -> Result<Vec<Calendar>, Error> {
    let calendar_refs = caldav::get_calendars(agent, username, password, base_url)?;
    let mut calendars = Vec::new();
    for calendar_ref in calendar_refs {
        calendars.push(Calendar {
            base_url: base_url.clone(),
            inner: calendar_ref,
        });
    }
    Ok(calendars)
}

/// Get all events in the given `Calendar`.
pub fn get_events(
    agent: Agent,
    username: &str,
    password: &str,
    calendar: &Calendar,
) -> Result<Vec<Event>, Error> {
    let event_refs = caldav::get_events(
        agent,
        username,
        password,
        &calendar.base_url,
        &calendar.inner,
    )?;
    let mut events = Vec::new();
    for event_ref in event_refs {
        let lines = ical::LineIterator::new(&event_ref.data);
        let ical = ical::Ical::parse(&lines)?;
        events.push(Event {
            url: event_ref.url.clone(),
            etag: event_ref.etag.clone(),
            ical,
        })
    }
    Ok(events)
}

/// Save the given event on the CalDAV server.
pub fn save_event(
    agent: Agent,
    username: &str,
    password: &str,
    event: Event,
) -> Result<Event, Error> {
    let event_ref = caldav::EventRef {
        data: event.ical.serialize(),
        etag: event.etag,
        url: event.url,
    };
    let event_ref = caldav::save_event(agent, username, password, event_ref)?;
    Ok(Event {
        etag: event_ref.etag,
        url: event_ref.url,
        ..event
    })
}

/// Delete the given event on the CalDAV server.
pub fn delete_event(
    agent: Agent,
    username: &str,
    password: &str,
    event: Event,
) -> Result<(), Error> {
    let event_ref = caldav::EventRef {
        data: event.ical.serialize(),
        etag: event.etag,
        url: event.url,
    };
    caldav::delete_event(agent, username, password, event_ref)?;
    Ok(())
}

/// A remote CalDAV calendar.
#[derive(Debug)]
pub struct Calendar {
    base_url: Url,
    inner: caldav::CalendarRef,
}

impl Calendar {
    pub fn url(&self) -> &Url {
        &self.inner.url
    }
    pub fn name(&self) -> &String {
        &self.inner.name
    }
}

/// A event in a CalDAV calendar.
#[derive(Debug)]
pub struct Event {
    etag: Option<String>,
    url: Url,
    ical: ical::Ical,
}

impl Event {
    /// The full url of this event.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Get the value of the given property or `None`.
    pub fn property(&self, name: &str) -> Option<&String> {
        self.ical
            .properties
            .iter()
            .find_map(|p| if p.name == name { Some(&p.value) } else { None })
    }

    /// Get all properties of this event.
    pub fn properties(&self) -> Vec<(String, String)> {
        self.ical
            .get("VEVENT")
            .unwrap()
            .properties
            .iter()
            .map(|p| (p.name.clone(), p.value.clone()))
            .collect()
    }
    pub fn etag(&self) -> Option<&String> {
        self.etag.as_ref()
    }
}

/// Errors that may occur during minicalav operations.
#[derive(Debug)]
pub enum Error {
    Ical(String),
    Caldav(String),
}

impl From<caldav::Error> for Error {
    fn from(e: caldav::Error) -> Self {
        Error::Ical(e.message)
    }
}

impl From<ical::Error> for Error {
    fn from(e: ical::Error) -> Self {
        Error::Caldav(e.message)
    }
}
