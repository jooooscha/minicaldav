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

use url::Url;
use log::*;

use crate::{calendar::CalendarRef, vcalendar_props::{VCalendarProps, ComponentResponse}, credentials::Credentials};

use reqwest::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, USER_AGENT},
    Client, Method,
};

use crate::xml_templates::build_create_calendar_xml;

use crate::errors::MiniCaldavError::{self, *};
use crate::xml_templates::*;

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
) -> Result<(String, xmltree::Element), MiniCaldavError> {
    let auth = credentials.as_header();

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
    let text = content.text().await?;

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
        Err(PathNotExists(format!("{:?}", prop_path)))
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
) -> Result<Url, MiniCaldavError> {
    let auth = credentials.as_header();

    let base_url = base_url.join("/.well-known/caldav")?;
    println!("base_url: {:?}", base_url);

    let response = client
        .get(base_url)
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
) -> Result<Url, MiniCaldavError> {
    let auth = credentials.as_header();

    let response = client
        .get(url.as_str())
        .header(USER_AGENT, "rust-minicaldav")
        .header(AUTHORIZATION, auth.clone())
        .send()
        .await?;


    println!("{:#?}", response);
    let response_url = response.error_for_status()?.url().clone();

    Ok(response_url)
}

/// Get the CalDAV principal URL for the given credentials from the caldav server.
pub async fn get_principal_url(
    client: &Client,
    credentials: &Credentials,
    url: Url,
) -> Result<Url, MiniCaldavError> {
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
    )
    .await?
    .0;
    Ok(url.join(&principal_url)?)
}

/// Get the homeset url for the given credentials from the caldav server.
pub async fn get_home_set_url(
    client: &Client,
    credentials: &Credentials,
    url: Url,
) -> Result<Url, MiniCaldavError> {
    let homeset_url = propfind_get(
        client,
        credentials,
        &url,
        HOMESET_REQUEST.to_string(),
        &["response", "propstat", "prop", "calendar-home-set", "href"],
        "0",
    )
    .await?
    .0;

    Ok(url.join(&homeset_url)?)
}


/// Get calendars for the given credentials.
pub async fn get_calendars(
    client: &Client,
    credentials: &Credentials,
    base_url: Url,
) -> Result<Vec<CalendarRef>, MiniCaldavError> {
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
    )
    .await;

    let root = match prop {
        Ok(p) => p.1,
        Err(_) => {
            propfind_get(
                client,
                credentials,
                &base_url,
                CALENDARS_QUERY.to_string(),
                &[],
                "1",
            )
            .await?
            .1
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
            let privileges: Vec<String> = response
                .get_child("propstat")
                .and_then(|e| e.get_child("prop"))
                .and_then(|e| e.get_child("current-user-privilege-set"))
                .map(|e| {
                    let mut list = Vec::new();
                    for privs in &e.children {
                        if let Some(p) = privs.as_element() {
                            for c in &p.children {
                                if let Some(c) = c.as_element() {
                                    list.push(c.name.clone());
                                }
                            }
                        }
                    }
                    list
                })
                .unwrap_or_else(Vec::new);

            let is_calendar = response
                .get_child("propstat")
                .and_then(|e| e.get_child("prop"))
                .and_then(|e| e.get_child("resourcetype"))
                .map(|e| e.get_child("calendar").is_some())
                .unwrap_or(false);

            let is_subscription = response
                .get_child("propstat")
                .and_then(|e| e.get_child("prop"))
                .and_then(|e| e.get_child("resourcetype"))
                .map(|e| e.get_child("subscribed").is_some())
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

            if !(is_calendar || is_subscription) || !supports_vevents {
                continue;
            }
            if let Some((href, name)) = href.and_then(|href| name.map(|name| (href, name))) {
                if let Ok(url) = base_url.join(&href) {
                    calendars.push(CalendarRef {
                        url,
                        name: name.to_string(),
                        color: color.map(|c| c.into()),
                        is_subscription,
                        privileges,
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


/// Get ICAL formatted events from the CalDAV server.
pub async fn get_components(
    client: &Client,
    credentials: &Credentials,
    base_url: Url,
    calendar_url: Url,
    start: Option<String>,
    end: Option<String>,
    expanded: bool,
) -> Result<Vec<ComponentResponse>, MiniCaldavError> {

    let xml = if expanded {
        &build_calendar_request_string(start, end, expanded)
    } else {
        CALENDAR_EVENTS_REQUEST // build_calendar_request_string(start, end);
    };

    let request = client
        .request(Method::from_bytes(b"REPORT").unwrap(), calendar_url)
        .header(USER_AGENT, "rust-minicaldav")
        .header(AUTHORIZATION, credentials.as_header())
        .header(CONTENT_TYPE, "application/xml; charset=utf-8")
        .header(ACCEPT, "text/xml, text/calendar")
        .header("Depth", "1")
        .body(xml.to_string());

    let content = request.send().await?.text().await?;

    // println!("content: {}", content);
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
                    events.push(ComponentResponse::new(url, etag, data.to_string()));
                } else {
                    error!("Could not parse url {}/{}", base_url, href)
                }
            }
        }
    }

    trace!("GET events response: {:#?}", events);
    Ok(events)
}

pub async fn get_ical_events(
    client: &Client,
    credentials: &Credentials,
    calendar_url: Url,
) -> Result<ComponentResponse, MiniCaldavError> {
    let auth = credentials.as_header();

    let response = client
        .get(calendar_url.clone())
        .header(USER_AGENT, "rust-minicaldav")
        .header(AUTHORIZATION, auth)
        .header(CONTENT_TYPE, "application/xml; charset=utf-8")
        .header(ACCEPT, "text/xml, text/calendar")
        .header("Depth", "1")
        .send().await?.text().await?;

    let event = ComponentResponse::new(calendar_url, None, response);
    Ok(event)
}

/// Save the given event on the CalDAV server.
/// If no event for the events url exist it will create a new event.
/// Otherwise this is an update operation.
pub async fn save_component(
    client: &Client,
    credentials: &Credentials,
    vcalendar_props: VCalendarProps,
) -> Result<VCalendarProps, MiniCaldavError> {

    let url = vcalendar_props.url().clone();

    let calendar = vcalendar_props.inner().clone();

    trace!("Saving component: {:#?}", calendar);

    // let data = component.inner().try_into_string()?;
    let data = calendar.to_string();

    let content_length = data.len();

    let response = client
        .put(url)
        .header(USER_AGENT, "rust-minicaldav")
        .header(CONTENT_TYPE, "text/calendar")
        .header(CONTENT_LENGTH, content_length.to_string())
        .header(AUTHORIZATION, &credentials.as_header())
        .body(data)
        .send()
        .await?;

    trace!("CALDAV response: {:#?}", response);

    let response = response.error_for_status()?;

    let etag = response
        .headers()
        .get("ETag")
        .map(|etag| etag.to_str().unwrap().to_string());

    let mut new_vcalendar_props = vcalendar_props;
    new_vcalendar_props.set_etag(etag);

    Ok(new_vcalendar_props)
}

/// Delete the given event from the CalDAV server.
pub async fn remove_component(
    client: &Client,
    credentials: &Credentials,
    event_url: Url,
) -> Result<(), MiniCaldavError> {

    let response = client
        .delete(event_url)
        .header(USER_AGENT, "rust-minicaldav")
        .header(AUTHORIZATION, &credentials.as_header())
        .send()
        .await?;

    response.error_for_status()?;

    Ok(())
}

/// Send a MKCOL request to create a new calendar collection
pub async fn create_calendar(
    client: &Client,
    credentials: &Credentials,
    base_url: &Url,
    calid: String,
    name: String,
    color: String,
) -> Result<(), MiniCaldavError> {
    let auth = credentials.as_header();

    let principal_url = get_principal_url(client, credentials, base_url.clone())
        .await
        .unwrap_or_else(|_| base_url.clone());

    let homeset_url = get_home_set_url(client, credentials, principal_url)
        .await
        .unwrap_or_else(|_| base_url.clone());

    let new_cal_url = homeset_url.join(&calid)?;

    let mkcol = Method::from_bytes(b"MKCOL").unwrap();

    let body = build_create_calendar_xml(name, color);

    trace!("Creating calendar: {}", body);
    let response = client
        .request(mkcol, new_cal_url)
        .header(USER_AGENT, "rust-minicaldav")
        .header(CONTENT_TYPE, "application/xml; charset=utf-8")
        .header(ACCEPT, "text/xml, text/calendar")
        .header(AUTHORIZATION, auth)
        .body(body)
        .send()
        .await?;

    let response = response.error_for_status();
    trace!("response.error_for_status(): {:?}", response);

    response?;

    Ok(())
}

pub async fn remove_calendar(
    client: &Client,
    credentials: &Credentials,
    base_url: &Url,
    calid: String,
) -> Result<(), MiniCaldavError> {
    let auth = credentials.as_header();

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

    response.error_for_status()?;

    Ok(())
}
