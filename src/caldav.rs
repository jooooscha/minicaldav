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

//! CalDAV client implementation using ureq.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use ureq::Agent;
use url::Url;

/// Send a PROPFIND to the given url using the given HTTP Basic authorization and search the result XML for a value.
/// # Arguments
/// - client: ureq Agent
/// - username: used for HTTP Basic auth
/// - password: used for HTTP Basic auth
/// - url: The caldav endpoint url
/// - body: The CalDAV request body to send via PROPFIND
/// - prop_path: The path in the response XML the get the XML text value from.
/// - depth: Value for the Depth field
pub fn propfind_get(
    client: Agent,
    username: &str,
    password: &str,
    url: &Url,
    body: &str,
    prop_path: &[&str],
    depth: &str,
) -> Result<(String, xmltree::Element), Error> {
    let auth = format!(
        "Basic {}",
        base64::encode(format!("{}:{}", username, password))
    );
    let content = client
        .request("PROPFIND", url.as_str())
        .set("Authorization", &auth)
        .set("Content-Type", "application/xml")
        .set("Depth", depth)
        .send_bytes(body.as_bytes())?
        .into_string()
        .map_err(|e| Error {
            kind: ErrorKind::Parsing,
            message: e.to_string(),
        })?;

    trace!("CalDAV propfind response: {:?}", content);
    let reader = content.as_bytes();

    let root = xmltree::Element::parse(reader)?;
    let mut element = &root;
    let mut searched = 0;
    for prop in prop_path {
        for e in &element.children {
            if let Some(child) = e.as_element() {
                if child.name == *prop {
                    searched += 1;
                    element = child;
                    break;
                }
            }
        }
    }

    if searched != prop_path.len() {
        Err(Error {
            kind: ErrorKind::Parsing,
            message: format!("Could not find data {:?} in PROPFIND response.", prop_path),
        })
    } else {
        Ok((
            element
                .get_text()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "".to_string()),
            root,
        ))
    }
}

pub static USER_PRINCIPAL_REQUEST: &str = r#"
    <d:propfind xmlns:d="DAV:">
       <d:prop>
           <d:current-user-principal />
       </d:prop>
    </d:propfind>
"#;

/// Get the CalDAV principal URL for the given credentials from the caldav server.
pub fn get_principal_url(
    client: Agent,
    username: &str,
    password: &str,
    url: &Url,
) -> Result<Url, Error> {
    let principal_url = propfind_get(
        client,
        username,
        password,
        url,
        USER_PRINCIPAL_REQUEST,
        &[
            "response",
            "propstat",
            "prop",
            "current-user-principal",
            "href",
        ],
        "0",
    )?
    .0;
    Ok(url.join(&principal_url)?)
}

pub static HOMESET_REQUEST: &str = r#"
    <d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" >
      <d:self/>
      <d:prop>
        <c:calendar-home-set />
      </d:prop>
    </d:propfind>
"#;

/// Get the homeset url for the given credentials from the caldav server.
pub fn get_home_set_url(
    client: Agent,
    username: &str,
    password: &str,
    url: &Url,
) -> Result<Url, Error> {
    let principal_url = get_principal_url(client.clone(), username, password, url)?;
    let homeset_url = propfind_get(
        client,
        username,
        password,
        &principal_url,
        HOMESET_REQUEST,
        &["response", "propstat", "prop", "calendar-home-set", "href"],
        "0",
    )?
    .0;
    Ok(url.join(&homeset_url)?)
}

pub static CALENDARS_REQUEST: &str = r#"
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" >
    <d:prop>
        <d:displayname />
        <d:resourcetype />
        <calendar-color xmlns="http://apple.com/ns/ical/" />
        <c:supported-calendar-component-set />
    </d:prop>
</d:propfind>
"#;

pub static CALENDARS_QUERY: &str = r#"
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
    <d:prop>
        <d:getetag />
        <d:displayname />
        <calendar-color xmlns="http://apple.com/ns/ical/" />
        <d:resourcetype />
        <c:supported-calendar-component-set />
    </d:prop>
    <c:filter>
        <c:comp-filter name="VCALENDAR" />
    </c:filter>
</c:calendar-query>
"#;

/// Get calendars for the given credentials.
pub fn get_calendars(
    client: Agent,
    username: &str,
    password: &str,
    base_url: &Url,
) -> Result<Vec<CalendarRef>, Error> {
    let mut calendars = Vec::new();
    let result = match get_home_set_url(client.clone(), username, password, base_url) {
        Ok(homeset_url) => propfind_get(
            client.clone(),
            username,
            password,
            &homeset_url,
            CALENDARS_REQUEST,
            &[],
            "1",
        ),
        Err(_e) => propfind_get(
            client.clone(),
            username,
            password,
            base_url,
            CALENDARS_REQUEST,
            &[],
            "1",
        ),
    };

    let root = if result.is_err() {
        propfind_get(
            client,
            username,
            password,
            base_url,
            CALENDARS_QUERY,
            &[],
            "1",
        )?
        .1
    } else {
        result?.1
    };

    for response in &root.children {
        if let Some(response) = response.as_element() {
            let name = response
                .get_child("propstat")
                .and_then(|e| e.get_child("prop"))
                .and_then(|e| e.get_child("displayname"))
                .and_then(|e| e.get_text());
            let color = response
                .get_child("propstat")
                .and_then(|e| e.get_child("prop"))
                .and_then(|e| e.get_child("calendar-color"))
                .and_then(|e| e.get_text());
            let is_calendar = response
                .get_child("propstat")
                .and_then(|e| e.get_child("prop"))
                .and_then(|e| e.get_child("resourcetype"))
                .map(|e| e.get_child("calendar").is_some())
                .unwrap_or(false);
            let supports_vevents = response
                .get_child("propstat")
                .and_then(|e| e.get_child("prop"))
                .and_then(|e| e.get_child("supported-calendar-component-set"))
                .map(|e| {
                    for c in &e.children {
                        if let Some(child) = c.as_element() {
                            if child.name == "comp" {
                                if let Some(name) = child.attributes.get("name") {
                                    if (name == "VEVENT") || (name == "VTODO") {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                    false
                })
                .unwrap_or(false);
            let href = response.get_child("href").and_then(|e| e.get_text());

            if !is_calendar || !supports_vevents {
                continue;
            }
            if let Some((href, name)) = href.and_then(|href| name.map(|name| (href, name))) {
                if let Ok(url) = base_url.join(&href) {
                    calendars.push(CalendarRef {
                        url,
                        name: name.to_string(),
                        color: color.map(|c| c.into()),
                    })
                } else {
                    error!("Could not parse url: {}/{}", base_url, href);
                }
            } else {
                continue;
            }
        }
    }
    Ok(calendars)
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CalendarRef {
    pub url: Url,
    pub name: String,
    pub color: Option<String>,
}

impl std::fmt::Debug for CalendarRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalendarRef")
            .field("url", &self.url.to_string())
            .field("name", &self.name)
            .finish()
    }
}

#[derive(Clone)]
pub struct EventRef {
    pub etag: Option<String>,
    pub url: Url,
    pub data: String,
}

impl std::fmt::Debug for EventRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventRef")
            .field("etag", &self.etag)
            .field("url", &self.url.to_string())
            .field("data", &self.data)
            .finish()
    }
}

pub static CALENDAR_EVENTS_REQUEST: &str = r#"
    <c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
        <d:prop>
            <d:getetag />
            <c:calendar-data />
        </d:prop>
        <c:filter>
            <c:comp-filter name="VCALENDAR">
                <c:comp-filter name="VEVENT" />
            </c:comp-filter>
        </c:filter>
    </c:calendar-query>
"#;

pub static CALENDAR_TODOS_REQUEST: &str = r#"
    <c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
        <d:prop>
            <d:getetag />
            <c:calendar-data />
        </d:prop>
        <c:filter>
            <c:comp-filter name="VCALENDAR">
                <c:comp-filter name="VTODO" />
            </c:comp-filter>
        </c:filter>
    </c:calendar-query>
"#;

/// Get ICAL formatted events from the CalDAV server.
pub fn get_events(
    client: Agent,
    username: &str,
    password: &str,
    base_url: &Url,
    calendar_url: &Url,
) -> Result<Vec<EventRef>, Error> {
    let auth = format!(
        "Basic {}",
        base64::encode(format!("{}:{}", username, password))
    );

    let content = client
        .request("REPORT", calendar_url.as_str())
        .set("Authorization", &auth)
        .set("Depth", "1")
        .set("Content-Type", "application/xml")
        .send_bytes(CALENDAR_EVENTS_REQUEST.as_bytes())?
        .into_string()
        .map_err(|e| Error {
            kind: ErrorKind::Parsing,
            message: e.to_string(),
        })?;

    trace!("Read CalDAV events: {:?}", content);
    let reader = content.as_bytes();

    let root = xmltree::Element::parse(reader)?;
    let mut events = Vec::new();
    for c in &root.children {
        if let Some(child) = c.as_element() {
            let href = child.get_child("href").and_then(|e| e.get_text());
            let etag = child
                .get_child("propstat")
                .and_then(|e| e.get_child("prop"))
                .and_then(|e| e.get_child("getetag"))
                .and_then(|e| e.get_text())
                .map(|e| e.to_string());
            let data = child
                .get_child("propstat")
                .and_then(|e| e.get_child("prop"))
                .and_then(|e| e.get_child("calendar-data"))
                .and_then(|e| e.get_text());
            if href.is_none() || etag.is_none() || data.is_none() {
                continue;
            }

            if let Some((href, data)) = href.and_then(|href| data.map(|data| (href, data))) {
                if let Ok(url) = base_url.join(&href) {
                    events.push(EventRef {
                        url,
                        data: data.to_string(),
                        etag,
                    })
                } else {
                    error!("Could not parse url {}/{}", base_url, href)
                }
            }
        }
    }

    Ok(events)
}

/// Get ICAL formatted todos from the CalDAV server.
pub fn get_todos(
    client: Agent,
    username: &str,
    password: &str,
    base_url: &Url,
    calendar_ref: &CalendarRef,
) -> Result<Vec<EventRef>, Error> {
    let auth = format!(
        "Basic {}",
        base64::encode(format!("{}:{}", username, password))
    );

    let content = client
        .request("REPORT", calendar_ref.url.as_str())
        .set("Authorization", &auth)
        .set("Depth", "1")
        .set("Content-Type", "application/xml")
        .send_bytes(CALENDAR_TODOS_REQUEST.as_bytes())?
        .into_string()
        .map_err(|e| Error {
            kind: ErrorKind::Parsing,
            message: e.to_string(),
        })?;

    trace!("Read CalDAV events: {:?}", content);
    let reader = content.as_bytes();

    let root = xmltree::Element::parse(reader)?;
    let mut todos = Vec::new();
    for c in &root.children {
        if let Some(child) = c.as_element() {
            let href = child.get_child("href").and_then(|e| e.get_text());
            let etag = child
                .get_child("propstat")
                .and_then(|e| e.get_child("prop"))
                .and_then(|e| e.get_child("getetag"))
                .and_then(|e| e.get_text())
                .map(|e| e.to_string());
            let data = child
                .get_child("propstat")
                .and_then(|e| e.get_child("prop"))
                .and_then(|e| e.get_child("calendar-data"))
                .and_then(|e| e.get_text());
            if href.is_none() || etag.is_none() || data.is_none() {
                continue;
            }

            if let Some((href, data)) = href.and_then(|href| data.map(|data| (href, data))) {
                if let Ok(url) = base_url.join(&href) {
                    todos.push(EventRef {
                        url,
                        data: data.to_string(),
                        etag,
                    })
                } else {
                    error!("Could not parse url {}/{}", base_url, href)
                }
            }
        }
    }

    Ok(todos)
}

/// Save the given event on the CalDAV server.
/// If no event for the events url exist it will create a new event.
/// Otherwise this is an update operation.
pub fn save_event(
    client: Agent,
    username: &str,
    password: &str,
    event_ref: EventRef,
) -> Result<EventRef, Error> {
    let auth = format!(
        "Basic {}",
        base64::encode(format!("{}:{}", username, password))
    );

    let response = client
        .put(event_ref.url.as_str())
        .set("Content-Type", "text/calendar")
        .set("Content-Length", &event_ref.data.len().to_string())
        .set("Authorization", &auth)
        .send(event_ref.data.as_bytes())?;

    if let Some(etag) = response.header("ETag") {
        Ok(EventRef {
            etag: Some(etag.into()),
            ..event_ref
        })
    } else {
        Ok(EventRef {
            etag: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|t| t.as_millis().to_string())
                    .unwrap_or_else(|_| "0".to_string()),
            ),
            ..event_ref
        })
    }
}

/// Delete the given event from the CalDAV server.
pub fn remove_event(
    client: Agent,
    username: &str,
    password: &str,
    event_ref: EventRef,
) -> Result<(), Error> {
    let auth = format!(
        "Basic {}",
        base64::encode(format!("{}:{}", username, password))
    );

    let _response = client
        .delete(event_ref.url.as_str())
        .set("Authorization", &auth)
        .call()?;

    Ok(())
}

/// Errors that may occur during CalDAV operations.
#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug)]
pub enum ErrorKind {
    Http,
    Parsing,
}

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        Self {
            kind: ErrorKind::Http,
            message: format!("{:?}", e),
        }
    }
}

impl From<xmltree::ParseError> for Error {
    fn from(e: xmltree::ParseError) -> Self {
        Self {
            kind: ErrorKind::Parsing,
            message: e.to_string(),
        }
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self {
            kind: ErrorKind::Parsing,
            message: e.to_string(),
        }
    }
}
