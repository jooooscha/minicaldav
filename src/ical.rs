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

//! Implementation of ICAL parsing.

use std::collections::HashMap;

/// A ICAL container. Can have properties or child ICAL containers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ical {
    pub name: String,
    pub properties: Vec<Property>,
    pub children: Vec<Ical>,
}

impl Ical {
    /// Create a new ical container. The name represents the value of the enclosing `BEGIN:` - `END:` pair.
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
    /// Get the child container which has the given name.
    pub fn get(&self, child_name: &str) -> Option<&Ical> {
        for c in &self.children {
            if c.name == child_name {
                return Some(c);
            }
        }
        None
    }
    /// Parse the given lines to an ICAL container.
    pub fn parse(lines: &LineIterator) -> Result<Self, Error> {
        let mut ical: Option<Ical> = None;
        let mut line_buffer = String::new();
        while let Some(line) = lines.next() {
            if line.trim().is_empty() {
                continue;
            }

            if let Some(line) = line.strip_prefix(' ') {
                if line_buffer.is_empty() {
                    if let Some(last_prop) = ical.as_mut().and_then(|ical| ical.properties.pop()) {
                        line_buffer.push_str(&last_prop.serialize());
                    }
                }
                line_buffer.push_str(line);
                continue;
            } else if !line.contains(':') && line_buffer.is_empty() {
                line_buffer.push_str(line);
                continue;
            } else if !line_buffer.is_empty() {
                let prop = Property::parse(&line_buffer)?;
                if let Some(ical) = ical.as_mut() {
                    ical.properties.push(prop);
                }
                line_buffer = String::new();
            }

            if !line.contains(':') {
                line_buffer.push_str(line);
                continue;
            }

            let prop = Property::parse(line)?;

            if ical.is_none() {
                if let Some(name) = prop.is("BEGIN") {
                    ical = Some(Ical::new(name.clone()));
                }
                continue;
            }
            if prop.is("BEGIN").is_some() {
                let child = Ical::parse(lines.decrement())?;
                if let Some(ical) = ical.as_mut() {
                    ical.children.push(child);
                }
                continue;
            }
            if let Some(name) = prop.is("END") {
                if ical.is_some() && Some(name) == ical.as_ref().map(|ical| &ical.name) {
                    if let Some(ical) = ical {
                        return Ok(ical);
                    }
                }
            }
            if let Some(ical) = ical.as_mut() {
                ical.properties.push(prop);
            } else {
                warn!("minicaldav wants to add a property but it does not seem to collect properties right now.")
            }
        }
        if ical.is_some() {
            Err(Error::new(format!(
                "Missing END:{:?}",
                ical.map(|i| i.name)
            )))
        } else {
            Err(Error::new("Invalid input".into()))
        }
    }
    /// Get ICAL formatted string of this container.
    pub fn serialize(&self) -> String {
        let mut string = String::new();
        string.push_str(&format!("BEGIN:{}\n", self.name));
        for prop in &self.properties {
            string.push_str(&prop.serialize());
            string.push('\n');
        }
        for child in &self.children {
            string.push_str(&child.serialize());
        }
        string.push_str(&format!("END:{}\n", self.name));
        string
    }
}

/// A utility struct used during ical parsing.
pub struct LineIterator<'a> {
    pos: std::cell::Cell<usize>,
    lines: Vec<&'a str>,
}

impl<'a> LineIterator<'a> {
    pub fn new(lines: &'a str) -> Self {
        Self {
            pos: std::cell::Cell::new(0),
            lines: lines.lines().collect(),
        }
    }
    fn next(&self) -> Option<&'a str> {
        let pos = self.pos.take();
        if pos >= self.lines.len() {
            self.pos.set(pos);
            None
        } else {
            self.pos.set(pos + 1);
            Some(self.lines[pos])
        }
    }
    fn decrement(&self) -> &Self {
        self.pos.set(self.pos.take() - 1);
        self
    }
}

/// An ICAL property. Currently only simple key-value properties are supported.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub value: String,
    pub attributes: HashMap<String, String>,
}

impl Property {
    /// Creates a new property.
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            attributes: HashMap::new(),
        }
    }

    /// Creates a new property with attributes
    pub fn new_with_attributes(name: &str, value: &str, attribs: Vec<(&str, &str)>) -> Self {
        let mut attributes = HashMap::new();
        for (k, v) in attribs {
            attributes.insert(k.into(), v.into());
        }
        Self {
            name: name.into(),
            value: value.into(),
            attributes,
        }
    }

    /// Parses the given input to a Property instance.
    pub fn parse(input: &str) -> Result<Self, Error> {
        let mut colon_index = None;
        let mut ignore_colon = false;
        for (i, c) in input.chars().enumerate() {
            if c == '"' {
                ignore_colon = !ignore_colon;
            } else if c == ':' && !ignore_colon {
                colon_index = Some(i);
                break;
            }
        }

        if let Some((name, value)) = colon_index.map(|i| input.split_at(i)) {
            // since we just splitted on ":" it must be the prefix of value
            if let Some(value) = value.strip_prefix(':') {
                let mut parts = name.split(';');

                if let Some(next) = parts.next() {
                    let mut property = Property::new(next.trim(), value);
                    for part in parts {
                        if let Some((k, v)) = part.split_once('=') {
                            property.attributes.insert(k.into(), v.into());
                        }
                    }
                    Ok(property)
                } else {
                    Err(Error::new(format!(
                        "The property [{:?}, {:?}] did not contain any text after split(';')",
                        name, value
                    )))
                }
            } else {
                Err(Error::new(format!(
                    "The property [{:?}, {:?}] was split on ':' but then ':' could not be stripped.", name, value
                )))
            }
        } else {
            Err(Error::new(format!(
                "Property '{}' does not match proper pattern",
                input
            )))
        }
    }

    /// Checks whether this property has the given name and if so, returns its value.
    pub fn is(&self, name: &str) -> Option<&String> {
        if self.name == name {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Serialized the property as ICAL formatted string.
    pub fn serialize(&self) -> String {
        if !self.attributes.is_empty() {
            format!(
                "{};{}:{}",
                self.name,
                self.attributes
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<String>>()
                    .join(";"),
                self.value
            )
        } else {
            format!("{}:{}", self.name, self.value)
        }
    }
}

impl From<(String, String)> for Property {
    fn from((name, value): (String, String)) -> Self {
        Self {
            name,
            value,
            attributes: HashMap::new(),
        }
    }
}

/// Errors that occur during ical parsing.
#[derive(Debug, PartialEq, Eq)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ical_calendar_with_properties() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//ZContent.net//Zap Calendar 1.0//EN
CALSCALE:GREGORIAN
METHOD:PUBLISH
END:VCALENDAR"#;
        assert_eq!(
            Ical::parse(&LineIterator::new(ical)),
            Ok(Ical {
                name: "VCALENDAR".to_string(),
                properties: vec![
                    Property::new("VERSION", "2.0"),
                    Property::new("PRODID", "-//ZContent.net//Zap Calendar 1.0//EN"),
                    Property::new("CALSCALE", "GREGORIAN"),
                    Property::new("METHOD", "PUBLISH"),
                ],
                children: vec![]
            })
        );
    }

    #[test]
    fn test_ical_calendar_with_events() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//ZContent.net//Zap Calendar 1.0//EN
CALSCALE:GREGORIAN
METHOD:PUBLISH
BEGIN:VEVENT
SUMMARY:Abraham Lincoln
UID:c7614cff-3549-4a00-9152-d25cc1fe077d
SEQUENCE:0
STATUS:CONFIRMED
TRANSP:TRANSPARENT
RRULE:FREQ=YEARLY;INTERVAL=1;BYMONTH=2;BYMONTHDAY=12
DTSTART:20080212
DTEND:20080213
DTSTAMP:20150421T141403
CATEGORIES:U.S. Presidents,Civil War People
LOCATION:Hodgenville\, Kentucky
GEO:37.5739497;-85.7399606
DESCRIPTION:Born February 12\, 1809\nSixteenth President (1861-1865)\n\n\n\nhttp://AmericanHistoryCalendar.com
URL:http://americanhistorycalendar.com/peoplecalendar/1,328-abraham-lincoln
END:VEVENT
END:VCALENDAR"#;
        let parsed = Ical::parse(&LineIterator::new(ical));
        // println!("{:#?}", parsed);
        assert_eq!(
            parsed,
            Ok(Ical {
                name: "VCALENDAR".into(),
                properties: vec![
                    Property::new("VERSION", "2.0"),
                    Property::new("PRODID", "-//ZContent.net//Zap Calendar 1.0//EN"),
                    Property::new("CALSCALE", "GREGORIAN"),
                    Property::new("METHOD", "PUBLISH"),
                ],
                children: vec![
                    Ical {
                        name: "VEVENT".into(),
                        properties: vec![
                            Property::new("SUMMARY", "Abraham Lincoln"),
                            Property::new("UID", "c7614cff-3549-4a00-9152-d25cc1fe077d"),
                            Property::new("SEQUENCE", "0"),
                            Property::new("STATUS", "CONFIRMED"),
                            Property::new("TRANSP", "TRANSPARENT"),
                            Property::new("RRULE", "FREQ=YEARLY;INTERVAL=1;BYMONTH=2;BYMONTHDAY=12"),
                            Property::new("DTSTART", "20080212"),
                            Property::new("DTEND", "20080213"),
                            Property::new("DTSTAMP", "20150421T141403"),
                            Property::new("CATEGORIES", "U.S. Presidents,Civil War People"),
                            Property::new("LOCATION", "Hodgenville\\, Kentucky"),
                            Property::new("GEO", "37.5739497;-85.7399606"),
                            Property::new("DESCRIPTION", "Born February 12\\, 1809\\nSixteenth President (1861-1865)\\n\\n\\n\\nhttp://AmericanHistoryCalendar.com"),
                            Property::new("URL", "http://americanhistorycalendar.com/peoplecalendar/1,328-abraham-lincoln"),
                        ],
                        children: vec![]
                    }
                ]
            })
        );
    }

    #[test]
    fn test_ical_calendar_with_event_with_long_description_and_linebreak() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//ZContent.net//Zap Calendar 1.0//EN
CALSCALE:GREGORIAN
METHOD:PUBLISH
BEGIN:VEVENT
SUMMARY:Abraham Lincoln
UID:c7614cff-3549-4a00-9152-d25cc1fe077d
SEQUENCE:0
STATUS:CONFIRMED
TRANSP:TRANSPARENT
RRULE:FREQ=YEARLY;INTERVAL=1;BYMONTH=2;BYMONTHDAY=12
DTSTART:20080212
DTEND:20080213
DTSTAMP:20150421T141403
CATEGORIES:U.S. Presidents,Civil War People
LOCATION:Hodgenville\, Kentucky
GEO:37.5739497;-85.7399606
DESCRIPTION:Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do
  eiusmod tempor incididunt ut labore et dolore magna aliqua
URL:http://americanhistorycalendar.com/peoplecalendar/1,328-abraham-lincoln
END:VEVENT
END:VCALENDAR
        "#;
        let parsed = Ical::parse(&LineIterator::new(ical));
        // println!("{:#?}", parsed);
        assert_eq!(
            parsed,
            Ok(Ical {
                name: "VCALENDAR".into(),
                properties: vec![
                    Property::new("VERSION", "2.0"),
                    Property::new("PRODID", "-//ZContent.net//Zap Calendar 1.0//EN"),
                    Property::new("CALSCALE", "GREGORIAN"),
                    Property::new("METHOD", "PUBLISH"),
                ],
                children: vec![
                    Ical {
                        name: "VEVENT".into(),
                        properties: vec![
                            Property::new("SUMMARY", "Abraham Lincoln"),
                            Property::new("UID", "c7614cff-3549-4a00-9152-d25cc1fe077d"),
                            Property::new("SEQUENCE", "0"),
                            Property::new("STATUS", "CONFIRMED"),
                            Property::new("TRANSP", "TRANSPARENT"),
                            Property::new("RRULE", "FREQ=YEARLY;INTERVAL=1;BYMONTH=2;BYMONTHDAY=12"),
                            Property::new("DTSTART", "20080212"),
                            Property::new("DTEND", "20080213"),
                            Property::new("DTSTAMP", "20150421T141403"),
                            Property::new("CATEGORIES", "U.S. Presidents,Civil War People"),
                            Property::new("LOCATION", "Hodgenville\\, Kentucky"),
                            Property::new("GEO", "37.5739497;-85.7399606"),
                            Property::new("DESCRIPTION", "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua"),
                            Property::new("URL", "http://americanhistorycalendar.com/peoplecalendar/1,328-abraham-lincoln"),
                        ],
                        children: vec![]
                    }
                ]
            })
        );
    }

    #[test]
    fn test_ical_with_max_line_linebreaks() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:asdf
BEGIN:VTIMEZONE
TZID:Europe/Berlin
LAST-MODIFIED:20220317T223602Z
TZURL:http://tzurl.org/zoneinfo-outlook/Europe/Berlin
X-LIC-LOCATION:Europe/Berlin
BEGIN:DAYLIGHT
TZNAME:CEST
TZOFFSETFROM:+0100
TZOFFSETTO:+0200
DTSTART:19700329T020000
RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU
END:DAYLIGHT
BEGIN:STANDARD
TZNAME:CET
TZOFFSETFROM:+0200
TZOFFSETTO:+0100
DTSTART:19701025T030000
RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU
END:STANDARD
END:VTIMEZONE
BEGIN:VEVENT
DTSTAMP:20220611T081002Z
ATTACH;FILENAME=arrow_up.svg;FMTTYPE=image/svg+xml;SIZE=2516;MANAGED-ID=1:h
 ttps://caldab.caldav.org/attachments/123456789012345/test.svg
ATTENDEE;CN=Fooo;PARTSTAT=NEEDS-ACTION;CUTYPE=INDIVIDUAL;EMAIL=foooooo.baaa
 aaar@fobar.com;X-CALENDARSERVER-DTSTAMP=20220611T081002Z:mailto:foooooo.ba
 aaaaar@fobar.com
ATTENDEE;CN=Foooooo Baaar;PARTSTAT=ACCEPTED;CUTYPE=INDIVIDUAL;EMAIL=foooo@b
 aaaaar.org;X-CALENDARSERVER-DTSTAMP=20220611T080833Z:mailto:foooo@baaaaar.
 org
CLASS:CONFIDENTIAL
CREATED:20220611T080833Z
DESCRIPTION:Lorem ipsum dolor sit amet\\, consectetur adipiscing elit\\, sed 
 do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad 
 minim veniam\\, quis nostrud exercitation ullamco laboris nisi ut aliquip e
 x ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptat
 e velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaec
 at cupidatat non proident\\, sunt in culpa qui officia deserunt mollit anim
 id est laborum.
DTSTART;TZID="(UTC-08:00) Pacific Time (US & Canada)";VALUE=DATE-TIME:2019
 1209T140000
DTEND;TZID="(UTC-08:00) Pacific Time (US & Canada)";VALUE=DATE-TIME:201912
 09T150000
LAST-MODIFIED:20220611T081002Z
ORGANIZER;CN=Foooooo Baaar;EMAIL=foooo@baaaaar.org:mailto:foooo@baaaaar.org
RRULE:FREQ=DAILY;COUNT=1
SEQUENCE:1
SUMMARY:Kaputtest
TRANSP:TRANSPARENT
UID:11111111-1111-1111-1111-111111111111
BEGIN:VALARM
TRIGGER;RELATED=START:-PT15M
UID:11111111-1111-1111-1111-111111111112
ACTION:DISPLAY
DESCRIPTION:reminder
END:VALARM
END:VEVENT
END:VCALENDAR"#;

        let parsed = Ical::parse(&LineIterator::new(ical));
        assert!(parsed.is_ok());
        let cal = parsed.unwrap();
        assert_eq!(
            cal.children[1].properties[0].serialize(),
            "DTSTAMP:20220611T081002Z"
        );
        let p1 = cal.children[1].properties[1].serialize();
        assert!(
            p1.starts_with("ATTACH")
                && p1.ends_with("https://caldab.caldav.org/attachments/123456789012345/test.svg")
        );
        let p2 = cal.children[1].properties[2].serialize();
        assert!(p2.starts_with("ATTENDEE") && p2.ends_with(":mailto:foooooo.baaaaaar@fobar.com"));
        let p6 = cal.children[1].properties[6].serialize();
        assert_eq!(
            p6, ("DESCRIPTION:Lorem ipsum dolor sit amet\\\\, consectetur adipiscing elit\\\\, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam\\\\, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident\\\\, sunt in culpa qui officia deserunt mollit animid est laborum.")
        );
        assert_eq!(cal.children[1].properties[7].value, "20191209T140000");
        assert_eq!(cal.children[1].properties[8].value, "20191209T150000");
    }

    #[test]
    fn test_serialize() {
        let expectation = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//ZContent.net//Zap Calendar 1.0//EN
CALSCALE:GREGORIAN
METHOD:PUBLISH
BEGIN:VEVENT
SUMMARY:Abraham Lincoln
UID:c7614cff-3549-4a00-9152-d25cc1fe077d
SEQUENCE:0
STATUS:CONFIRMED
TRANSP:TRANSPARENT
RRULE:FREQ=YEARLY;INTERVAL=1;BYMONTH=2;BYMONTHDAY=12
DTSTART:20080212
DTEND:20080213
DTSTAMP:20150421T141403
CATEGORIES:U.S. Presidents,Civil War People
LOCATION:Hodgenville\, Kentucky
GEO:37.5739497;-85.7399606
DESCRIPTION:Born February 12\, 1809\nSixteenth President (1861-1865)\n\n\n\nhttp://AmericanHistoryCalendar.com
URL:http://americanhistorycalendar.com/peoplecalendar/1,328-abraham-lincoln
END:VEVENT
END:VCALENDAR
"#;

        let ical = Ical {
            name: "VCALENDAR".into(),
            properties: vec![
                Property::new("VERSION", "2.0"),
                Property::new("PRODID", "-//ZContent.net//Zap Calendar 1.0//EN"),
                Property::new("CALSCALE", "GREGORIAN"),
                Property::new("METHOD", "PUBLISH"),
            ],
            children: vec![
                Ical {
                    name: "VEVENT".into(),
                    properties: vec![
                        Property::new("SUMMARY", "Abraham Lincoln"),
                        Property::new("UID", "c7614cff-3549-4a00-9152-d25cc1fe077d"),
                        Property::new("SEQUENCE", "0"),
                        Property::new("STATUS", "CONFIRMED"),
                        Property::new("TRANSP", "TRANSPARENT"),
                        Property::new("RRULE", "FREQ=YEARLY;INTERVAL=1;BYMONTH=2;BYMONTHDAY=12"),
                        Property::new("DTSTART", "20080212"),
                        Property::new("DTEND", "20080213"),
                        Property::new("DTSTAMP", "20150421T141403"),
                        Property::new("CATEGORIES", "U.S. Presidents,Civil War People"),
                        Property::new("LOCATION", "Hodgenville\\, Kentucky"),
                        Property::new("GEO", "37.5739497;-85.7399606"),
                        Property::new("DESCRIPTION", "Born February 12\\, 1809\\nSixteenth President (1861-1865)\\n\\n\\n\\nhttp://AmericanHistoryCalendar.com"),
                        Property::new("URL", "http://americanhistorycalendar.com/peoplecalendar/1,328-abraham-lincoln"),
                    ],
                    children: vec![]
                }
            ]
        };

        assert_eq!(ical.serialize(), expectation);
    }
}
