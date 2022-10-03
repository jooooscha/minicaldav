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

use std::collections::HashMap;

use crate::caldav;
use crate::ical;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
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

/// Get all todos in the given `Calendar`.
/// This function returns a tuple of all todos that could be parsed and all todos that couldn't.
/// If anything besides parsing the todo data fails, an Err will be returned.
pub fn get_todos(
    agent: Agent,
    username: &str,
    password: &str,
    calendar: &Calendar,
) -> Result<(Vec<Event>, Vec<Error>), Error> {
    let todo_refs = caldav::get_todos(
        agent,
        username,
        password,
        &calendar.base_url,
        &calendar.inner,
    )?;
    let mut todos = Vec::new();
    let mut errors = Vec::new();
    for todo_ref in todo_refs {
        let lines = ical::LineIterator::new(&todo_ref.data);
        match ical::Ical::parse(&lines) {
            Ok(ical) => todos.push(Event {
                url: todo_ref.url.clone(),
                etag: todo_ref.etag.clone(),
                ical,
            }),
            Err(e) => errors.push(Error::Ical(format!(
                "Could not parse todo {}: {:?}",
                todo_ref.data, e
            ))),
        }
    }
    Ok((todos, errors))
}

/// Get all events in the given `Calendar`.
/// This function returns a tuple of all events that could be parsed and all events that couldn't.
/// If anything besides parsing the event data fails, an Err will be returned.
pub fn get_events(
    agent: Agent,
    username: &str,
    password: &str,
    calendar: &Calendar,
) -> Result<(Vec<Event>, Vec<Error>), Error> {
    let event_refs = caldav::get_events(
        agent,
        username,
        password,
        &calendar.base_url,
        &calendar.inner,
    )?;
    let mut events = Vec::new();
    let mut errors = Vec::new();
    for event_ref in event_refs {
        let lines = ical::LineIterator::new(&event_ref.data);
        match ical::Ical::parse(&lines) {
            Ok(ical) => events.push(Event {
                url: event_ref.url.clone(),
                etag: event_ref.etag.clone(),
                ical,
            }),
            Err(e) => errors.push(Error::Ical(format!(
                "Could not parse event {}: {:?}",
                event_ref.data, e
            ))),
        }
    }
    Ok((events, errors))
}

/// Save the given event on the CalDAV server.
pub fn save_event(
    agent: Agent,
    username: &str,
    password: &str,
    mut event: Event,
) -> Result<Event, Error> {
    for prop in &mut event.ical.properties {
        if prop.name == "SEQUENCE" {
            if let Ok(num) = prop.value.parse::<i64>() {
                prop.value = format!("{}", num + 1);
            }
        }
    }

    let event_ref = caldav::EventRef {
        data: event.ical.serialize(),
        etag: None,
        url: event.url,
    };
    let event_ref = caldav::save_event(agent, username, password, event_ref)?;
    Ok(Event {
        etag: event_ref.etag,
        url: event_ref.url,
        ..event
    })
}

/// Remove the given event on the CalDAV server.
pub fn remove_event(
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
    caldav::remove_event(agent, username, password, event_ref)?;
    Ok(())
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    pub fn color(&self) -> Option<&String> {
        self.inner.color.as_ref()
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// An event in a CalDAV calendar.
#[derive(Debug, Clone, Eq, PartialEq)]
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

    fn get_property(&self, name: &str, datatype: &str) -> Option<Property> {
        self.ical.get(datatype).and_then(|ical| {
            ical.properties.iter().find_map(|p| {
                if p.name == name {
                    Some(Property::from(p.clone()))
                } else {
                    None
                }
            })
        })
    }

    /// Get the property of the given name or `None`.
    pub fn pop_property(&mut self, name: &str) -> Option<Property> {
        self.ical.get_mut("VEVENT").and_then(|ical| {
            let index = ical.properties.iter().enumerate().find_map(|(i, p)| {
                if p.name == name {
                    Some(i)
                } else {
                    None
                }
            });

            if let Some(index) = index {
                Some(Property::from(ical.properties.remove(index)))
            } else {
                None
            }
        })
    }

    pub fn add(&mut self, property: Property) {
        if let Some(ical) = self.ical.get_mut("VEVENT") {
            ical.properties.push(property.into());
        }
    }

    pub fn property(&self, name: &str) -> Option<Property> {
        self.get_property(name, "VEVENT")
    }

    /// Get the property of the given name or `None`.
    pub fn property_todo(&self, name: &str) -> Option<Property> {
        self.get_property(name, "VTODO")
    }

    /// Get the value of the given property name or `None`.
    pub fn get(&self, name: &str) -> Option<&String> {
        self.ical.get("VEVENT").and_then(|ical| {
            ical.properties
                .iter()
                .find_map(|p| if p.name == name { Some(&p.value) } else { None })
        })
    }

    /// Get all properties of this event.
    fn get_properties(&self, datatype: &str) -> Vec<(&String, &String)> {
        self.ical
            .get(datatype)
            .map(|ical| {
                ical.properties
                    .iter()
                    .map(|p| (&p.name, &p.value))
                    .collect::<Vec<(&String, &String)>>()
            })
            .unwrap_or_else(|| {
                error!("Could not get properties: No VEVENT section.");
                Vec::new()
            })
    }

    pub fn into_properties(self) -> Vec<Property> {
        for ical in self.ical.children {
            if ical.name == "VEVENT" {
                return ical.properties.into_iter().map(Property::from).collect();
            }
        }
        Vec::new()
    }

    /// Get all properties of this event.
    pub fn properties(&self) -> Vec<(&String, &String)> {
        self.get_properties("VEVENT")
    }

    /// Get all properties of this todo.
    pub fn properties_todo(&self) -> Vec<(&String, &String)> {
        self.get_properties("VTODO")
    }

    pub fn etag(&self) -> Option<&String> {
        self.etag.as_ref()
    }

    pub fn builder(url: Url) -> EventBuilder {
        EventBuilder {
            url,
            etag: None,
            properties: vec![],
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    name: String,
    value: String,
    attributes: HashMap<String, String>,
}

impl Property {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn value(&self) -> &String {
        &self.value
    }

    pub fn into_value(self) -> String {
        self.value
    }

    pub fn attribute(&self, name: &str) -> Option<&String> {
        self.attributes.get(name)
    }
}

impl From<ical::Property> for Property {
    fn from(p: ical::Property) -> Self {
        Self {
            name: p.name,
            value: p.value,
            attributes: p.attributes,
        }
    }
}

impl From<Property> for ical::Property {
    fn from(p: Property) -> Self {
        Self {
            name: p.name,
            value: p.value,
            attributes: p.attributes,
        }
    }
}

/// Errors that may occur during minicalav operations.
#[derive(Debug)]
pub enum Error {
    Ical(String),
    Caldav(String),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Ical(msg) => write!(f, "CalDAV Error: '{}'", msg),
            Error::Caldav(msg) => write!(f, "ICAL Error: '{}'", msg),
        }
    }
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

#[derive(Debug)]
pub struct EventBuilder {
    url: Url,
    etag: Option<String>,
    properties: Vec<ical::Property>,
}

impl EventBuilder {
    fn build_event(self, name: String) -> Event {
        Event {
            etag: self.etag,
            url: self.url,
            ical: ical::Ical {
                name: "VCALENDAR".into(),
                properties: vec![],
                children: vec![ical::Ical {
                    name,
                    properties: self.properties,
                    children: vec![],
                }],
            },
        }
    }

    pub fn build(self) -> Event {
        self.build_event("VEVENT".into())
    }

    pub fn build_todo(self) -> Event {
        self.build_event("VTODO".into())
    }

    pub fn etag(mut self, etag: Option<String>) -> Self {
        self.etag = etag;
        self
    }

    pub fn uid(mut self, value: String) -> Self {
        self.properties.push(ical::Property {
            name: "UID".to_string(),
            value,
            attributes: HashMap::new(),
        });
        self
    }

    pub fn timestamp(mut self, value: String) -> Self {
        self.properties.push(ical::Property {
            name: "DTSTAMP".to_string(),
            value,
            attributes: HashMap::new(),
        });
        self
    }

    pub fn summary(mut self, value: String) -> Self {
        self.properties.push(ical::Property {
            name: "SUMMARY".to_string(),
            value,
            attributes: HashMap::new(),
        });
        self
    }

    pub fn priority(mut self, value: String) -> Self {
        self.properties.push(ical::Property {
            name: "PRIORITY".to_string(),
            value,
            attributes: HashMap::new(),
        });
        self
    }

    pub fn duedate(mut self, value: String) -> Self {
        self.properties.push(ical::Property {
            name: "DUE".to_string(),
            value,
            attributes: HashMap::new(),
        });
        self
    }

    pub fn status(mut self, value: String) -> Self {
        self.properties.push(ical::Property {
            name: "STATUS".to_string(),
            value,
            attributes: HashMap::new(),
        });
        self
    }

    pub fn generic(mut self, name: String, value: String) -> Self {
        self.properties.push(ical::Property {
            name,
            value,
            attributes: HashMap::new(),
        });
        self
    }

    pub fn location(mut self, value: Option<String>) -> Self {
        if let Some(value) = value {
            self.properties.push(ical::Property {
                name: "LOCATION".to_string(),
                value,
                attributes: HashMap::new(),
            });
        }
        self
    }

    pub fn start(mut self, value: String, attributes: Vec<(&str, &str)>) -> Self {
        let mut attribs = HashMap::new();
        for (k, v) in attributes {
            attribs.insert(k.into(), v.into());
        }
        self.properties.push(ical::Property {
            name: "DTSTART".to_string(),
            value,
            attributes: attribs,
        });
        self
    }

    pub fn end(mut self, value: String, attributes: Vec<(&str, &str)>) -> Self {
        let mut attribs = HashMap::new();
        for (k, v) in attributes {
            attribs.insert(k.into(), v.into());
        }
        self.properties.push(ical::Property {
            name: "DTEND".to_string(),
            value,
            attributes: attribs,
        });
        self
    }

    pub fn description(mut self, value: Option<String>) -> Self {
        if let Some(value) = value {
            self.properties.push(ical::Property {
                name: "DESCRIPTION".to_string(),
                value,
                attributes: HashMap::new(),
            });
        }
        self
    }

    pub fn rrule(mut self, value: Option<String>) -> Self {
        if let Some(value) = value {
            self.properties.push(ical::Property {
                name: "RRULE".to_string(),
                value,
                attributes: HashMap::new(),
            });
        }
        self
    }
}
