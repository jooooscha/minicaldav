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
use url::Url;

use crate::credentials::Credentials;

use reqwest::{header::{ACCEPT, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, USER_AGENT}, Client, Method};

use crate::xml_templates::build_create_calendar_xml;


/// Send a PROPFIND to the given url using the given HTTP Basic authorization and search the result XML for a value.
/// # Arguments
/// - client: ureq Agent
/// - username: used for HTTP Basic auth
/// - password: used for HTTP Basic auth
/// - url: The caldav endpoint url
/// - body: The CalDAV request body to send via PROPFIND
/// - prop_path: The path in the response XML the get the XML text value from.
/// - depth: Value for the Depth field
pub async fn propfind_get(
    client: &Client,
    credentials: &Credentials,
    url: &Url,
    body: String,
    prop_path: &[&str],
    depth: &str,
) -> Result<(String, xmltree::Element), Error> {
    let auth = get_auth_header(credentials);

    let propfind = Method::from_bytes(b"PROPFIND").unwrap();

    let content = client
        .request(propfind, url.as_str())
        .header(USER_AGENT, "rust-minicaldav")
        .header(CONTENT_TYPE, "application/xml; charset=utf-8")
        .header(ACCEPT, "text/xml, text/calendar")
        .header(AUTHORIZATION, auth)
        .header("Depth", depth)
        .body(body)
        .send()
        .await?;

    trace!("CalDAV propfind response: {:?}", content);
    let text = content
        .text()
        .await?;

    let root = xmltree::Element::parse(text.as_bytes())?;
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

/// Discover the content url of the DAV server
pub async fn discover_url(
    client: &Client,
    credentials: &Credentials,
    base_url: Url,
) -> Result<Url, Error> {
    let auth = get_auth_header(credentials);

    let base_url = base_url.join("/.well-known/caldav")?;
    println!("base_url: {:?}", base_url);

    let response = client.get(base_url)
        .header(AUTHORIZATION, &auth)
        .send()
        .await?;

    Ok(response.url().clone())
}

/// Simple connection check to the DAV server
/// Returns the final url.
/// This can be used for content url bootstrapping
pub async fn check_connetion(
    client: &Client,
    credentials: &Credentials,
    url: &Url,
) -> Result<Url, Error> {
    let auth = get_auth_header(credentials);

    let response = client.get(url.as_str())
        .header(USER_AGENT, "rust-minicaldav")
        .header(AUTHORIZATION, auth.clone());

    let response = response.send()
        .await?;

    match response.error_for_status() {
        Ok(resp) => Ok(resp.url().clone()),
        Err(err) => Err(Error { kind: ErrorKind::Http, message: err.to_string() }),
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
pub async fn get_principal_url(
    client: &Client,
    credentials: &Credentials,
    url: Url,
) -> Result<Url, Error> {
    let principal_url = propfind_get(
        client,
        credentials,
        &url,
        USER_PRINCIPAL_REQUEST.to_string(),
        &[
            "response",
            "propstat",
            "prop",
            "current-user-principal",
            "href",
        ],
        "0",
    ).await?.0;
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
pub async fn get_home_set_url(
    client: &Client,
    credentials: &Credentials,
    url: Url
) -> Result<Url, Error> {

    let homeset_url = propfind_get(
        client,
        credentials,
        &url,
        HOMESET_REQUEST.to_string(),
        &["response", "propstat", "prop", "calendar-home-set", "href"],
        "0",
    ).await?
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
pub async fn get_calendars(
    client: &Client,
    credentials: &Credentials,
    base_url: Url,
) -> Result<Vec<CalendarRef>, Error> {
    let mut calendars = Vec::new();

    let principal_url = get_principal_url(client, credentials, base_url.clone())
        .await
        .unwrap_or_else(|_| base_url.clone());

    let homeset_url = get_home_set_url(client, credentials, principal_url)
        .await
        .unwrap_or_else(|_| base_url.clone());

    let prop = propfind_get(
        client,
        credentials,
        &homeset_url,
        CALENDARS_REQUEST.to_string(),
        &[],
        "1",
    ).await;

    let root = match prop {
        Ok(p) => p.1,
        Err(_) => {
            propfind_get(
                client,
                credentials,
                &base_url,
                CALENDARS_QUERY.to_string(),
                &[],
                "1"
            ).await?.1
        }
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
#[derive(Clone)]
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
            // <c:calendar-data>
            //     <c:expand start="20000103T000000Z" end="21000105T000000Z"/>
            // </c:calendar-data>
                //<c:comp-filter name="VEVENT">
                //    <c:time-range start="20250103T000000Z" end="20260105T000000Z"/>
                //</c:comp-filter>

fn build_calendar_request_string(start: Option<String>, end: Option<String>, expanded: bool) -> String {

   let start = start.as_deref().unwrap_or("20000103T000000Z");
   let end = end.as_deref().unwrap_or("21000105T000000Z");

   let req = if expanded {
       format!(r#"<c:calendar-data>
          <c:expand start="{}" end="{}"/>
       </c:calendar-data>"#, start, end)
   } else {
       format!(r#"<c:calendar-data>
          <c:limit-recurrance-set start="{}" end="{}"/>
       </c:calendar-data>"#, start, end)
   };

   let range =
           format!(r#"<c:comp-filter name="VEVENT">
                   <c:time-range start="{}" end="{}"/>
               </c:comp-filter>"#, start, end);

   format!(r#"
    <c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
        <d:prop>
            <d:getetag />
            {}
        </d:prop>
        <c:filter>
            <c:comp-filter name="VCALENDAR">
                {}
            </c:comp-filter>
        </c:filter>
    </c:calendar-query>
   "#, req, range)
}


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
pub async fn get_events(
    client: &Client,
    credentials: &Credentials,
    base_url: Url,
    calendar_url: Url,
    start: Option<String>,
    end: Option<String>,
    expanded: bool
) -> Result<Vec<EventRef>, Error> {

    let auth = get_auth_header(credentials);

    let xml = if expanded {
        &build_calendar_request_string(start, end, expanded)
    } else {
        CALENDAR_EVENTS_REQUEST // build_calendar_request_string(start, end);
    };

    let request = client
        .request(Method::from_bytes(b"REPORT").unwrap(), calendar_url)
        .header(USER_AGENT, "rust-minicaldav")
        .header(AUTHORIZATION, auth)
        .header(CONTENT_TYPE, "application/xml; charset=utf-8")
        .header(ACCEPT, "text/xml, text/calendar")
        .header("Depth", "1")
        .body(xml.to_string());

    let content = request
        .send()
        .await?
        .text()
        .await?;

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

fn get_auth_header(credentials: &Credentials) -> String {
    match credentials {
        Credentials::Basic(username, password) => {
            format!(
                "Basic {}",
                base64::encode(format!("{}:{}", username, password))
                // username, password
            )
        }
        Credentials::Bearer(token) => format!("Bearer {}", token),
    }
}

/// Get ICAL formatted todos from the CalDAV server.
pub async fn get_todos(
    client: &Client,
    credentials: &Credentials,
    base_url: &Url,
    calendar_ref: &CalendarRef,
) -> Result<Vec<EventRef>, Error> {
    let auth = get_auth_header(credentials);

    let report = Method::from_bytes(b"REPORT").unwrap();

    let content = client
        .request(report, calendar_ref.url.as_str())
        .header(USER_AGENT, "rust-minicaldav")
        .header(AUTHORIZATION, &auth)
        .header("Depth", "1")
        .header(CONTENT_TYPE, "application/xml; chatset=utf-8")
        .body(CALENDAR_TODOS_REQUEST.as_bytes())
        .send()
        .await?
        .text()
        .await?;

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
pub async fn save_event(
    client: &Client,
    credentials: &Credentials,
    event_ref: EventRef,
) -> Result<EventRef, Error> {
    let auth = get_auth_header(credentials);

    let EventRef { data, url, .. } = event_ref.clone();

    let content_length = data.len();

    let response = client.put(url)
        .header(USER_AGENT, "rust-minicaldav")
        .header(CONTENT_TYPE, "text/calendar")
        .header(CONTENT_LENGTH, content_length.to_string())
        .header(AUTHORIZATION, &auth)
        .body(data)
        .send()
        .await?;

    let etag = response.headers().get("ETag")
        .map(|etag| etag.to_str().unwrap().to_string());

    let event_ref = EventRef {
        etag,
        ..event_ref
    };

    Ok(event_ref)
}

/// Delete the given event from the CalDAV server.
pub async fn remove_event(
    client: &Client,
    credentials: &Credentials,
    event_ref: EventRef,
) -> Result<(), Error> {
    let auth = get_auth_header(credentials);

    let _response = client
        .delete(event_ref.url.as_str())
        .header(USER_AGENT, "rust-minicaldav")
        .header(AUTHORIZATION, &auth)
        .send()
        .await?;

    Ok(())
}


/// Send a MKCOL request to create a new calendar collection
pub async fn create_calendar(
    client: &Client,
    credentials: &Credentials,
    base_url: &Url,
    calid: String,
    name: String,
    color: String
) -> Result<(), Error> {
    let auth = get_auth_header(credentials);

    let principal_url = get_principal_url(client, credentials, base_url.clone())
        .await
        .unwrap_or_else(|_| base_url.clone());

    let homeset_url = get_home_set_url(client, credentials, principal_url)
        .await
        .unwrap_or_else(|_| base_url.clone());

    let new_cal_url = homeset_url.join(&calid)?;

    let mkcol = Method::from_bytes(b"MKCOL").unwrap();

    let body = build_create_calendar_xml(name, color);

    let response = client
        .request(mkcol, new_cal_url)
        .header(USER_AGENT, "rust-minicaldav")
        .header(CONTENT_TYPE, "application/xml; charset=utf-8")
        .header(ACCEPT, "text/xml, text/calendar")
        .header(AUTHORIZATION, auth)
        .body(body)
        .send()
        .await?;

    Ok(())
}

pub async fn remove_calendar(
    client: &Client,
    credentials: &Credentials,
    base_url: &Url,
    calid: String,
) -> Result<(), Error> {
    let auth = get_auth_header(credentials);

    let principal_url = get_principal_url(client, credentials, base_url.clone())
        .await
        .unwrap_or_else(|_| base_url.clone());

    let homeset_url = get_home_set_url(client, credentials, principal_url)
        .await
        .unwrap_or_else(|_| base_url.clone());

    let cal_url = homeset_url.join(&calid)?;

    let response = client
        .delete(cal_url)
        .header(USER_AGENT, "rust-minicaldav")
        .header(CONTENT_TYPE, "application/xml; charset=utf-8")
        .header(AUTHORIZATION, auth)
        .send()
        .await?;

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

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
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
