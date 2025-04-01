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

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write as _;
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
        self.children.iter().find(|&c| c.name == child_name)
    }
    /// Get the child container which has the given name.
    pub fn get_mut(&mut self, child_name: &str) -> Option<&mut Ical> {
        self.children.iter_mut().find(|c| c.name == child_name)
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
                    ical = Some(Ical::new(name.trim().to_string()));
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
                if ical.is_some()
                    && Some(name.as_str()) == ical.as_ref().map(|ical| ical.name.trim())
                {
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
                "Missing END:{}",
                ical.map(|i| i.name).unwrap_or_default()
            )))
        } else {
            Err(Error::new("Invalid input".into()))
        }
    }

    /// Get ICAL formatted string of this container.
    pub fn serialize(&self) -> String {
        let mut string = String::new();
        let _ = writeln!(string, "BEGIN:{}", self.name);
        for prop in &self.properties {
            string.push_str(&prop.serialize());
            string.push('\n');
        }
        for child in &self.children {
            string.push_str(&child.serialize());
        }
        let _ = writeln!(string, "END:{}", self.name);
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
        // Find position of the first colon (separation between name and value) that is not
        // enclosed in `"`.
        let mut name = String::new();
        let mut value = String::new();
        let mut read_value = false;
        let mut ignore_colon = false;
        for c in input.chars() {
            if c == '"' {
                ignore_colon = !ignore_colon;
            } else if c == ':' && !ignore_colon && !read_value {
                read_value = true;
                continue;
            }
            if read_value {
                value.push(c);
            } else {
                name.push(c);
            }
        }
        let mut parts = name.split(';');
        if let Some(next) = parts.next() {
            let mut property = Property::new(next.trim(), &value);
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

    /// This test uses a two-byte char to check whether they are correctly treated.
    #[test]
    fn test_ical_with_umlaut_in_property() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Open-Xchange//7.10.6-Rev16//EN
BEGIN:VEVENT
ATTENDEE;CN="Alice Rüd";RSVP=TRUE;EMAIL=alice@example.com:mailto:alice@example.com
DTEND;TZID=Europe/Berlin:20210808T220000
DTSTART;TZID=Europe/Berlin:20210808T200000
SEQUENCE:0
STATUS:CONFIRMED
SUMMARY:Let's go and find something to eat
TRANSP:OPAQUE
UID:5dd6eacd-c03d-4a47-b2b4-327486fc000c
END:VEVENT
END:VCALENDAR"#;

        let parsed = Ical::parse(&LineIterator::new(ical));
        assert!(parsed.is_ok());
        let cal = parsed.unwrap();
        assert_eq!(
            cal.children[0].properties[0],
            Property::new_with_attributes(
                "ATTENDEE",
                "mailto:alice@example.com",
                vec![
                    ("RSVP", "TRUE"),
                    ("EMAIL", "alice@example.com"),
                    ("CN", "\"Alice Rüd\"")
                ]
            )
        );
    }

    #[test]
    fn test_event_with_long_location() {
        // let ical = "BEGIN:VCALENDAR\nCALSCALE:GREGORIAN\nPRODID:-//Apple Inc.//iOS 13.2//EN\nVERSION:2.0\nBEGIN:VTIMEZONE\nTZID:America/Vancouver\nBEGIN:DAYLIGHT\nDTSTART:20070311T020000\nRRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=2SU\nTZNAME:GMT-7\nTZOFFSETFROM:-0800\nTZOFFSETTO:-0700\nEND:DAYLIGHT\nBEGIN:STANDARD\nDTSTART:20071104T020000\nRRULE:FREQ=YEARLY;BYMONTH=11;BYDAY=1SU\nTZNAME:GMT-8\nTZOFFSETFROM:-0700\nTZOFFSETTO:-0800\nEND:STANDARD\nEND:VTIMEZONE\nBEGIN:VEVENT\nCREATED:20191002T181641Z\nDTEND;TZID=America/Vancouver:20191014T090000\nDTSTAMP:20191014T050818Z\nDTSTART;TZID=America/Vancouver:20191014T070000\nLAST-MODIFIED:20191002T181641Z\nLOCATION:312 Boren Ave South\\nSeattle WA 98144\\nUSA\nSEQUENCE:0\nSUMMARY:Truline Swaybar Bushing Installation\nTRANSP:OPAQUE\nUID:2DC25870-241F-4279-9720-AB26C8DA5A60\nURL;VALUE=URI:\nX-APPLE-STRUCTURED-LOCATION;VALUE=URI;X-ADDRESS=312 Boren Ave South\\\\nSe\n attle WA 98144\\\\nUSA;X-APPLE-ABUID=Doug Tate Tru-Line’s Work;X-APPLE-MAP\n KIT-HANDLE=CAESjQIIwjsaEglN7OsKx8xHQBF/pyRGKpRewCKEAQoNVW5pdGVkIFN0YXRlc\n xICVVMaCldhc2hpbmd0b24iAldBKgRLaW5nMgdTZWF0dGxlOgU5ODE0NEIIQXRsYW50aWNSC\n 0JvcmVuIEF2ZSBTWgMzMTJiDzMxMiBCb3JlbiBBdmUgU4oBEENlbnRyYWwgRGlzdHJpY3SKA\n QhBdGxhbnRpYyoPMzEyIEJvcmVuIEF2ZSBTMg8zMTIgQm9yZW4gQXZlIFMyElNlYXR0bGUsI\n FdBICA5ODE0NDINVW5pdGVkIFN0YXRlczg5QARaJAoiEhIJTezrCsfMR0ARf6ckRiqUXsAYw\n jsgodmfx+vg0YSuAQ==;X-APPLE-RADIUS=22.28845908357249;X-APPLE-REFERENCEFR\n AME=1;X-TITLE=312 Boren Ave South\\\\nSeattle WA 98144\\\\nUSA:geo:47.599824\n ,-122.315080\nX-APPLE-TRAVEL-ADVISORY-BEHAVIOR:DISABLED\nBEGIN:VALARM\nACTION:DISPLAY\nDESCRIPTION:Reminder\nTRIGGER:-PT30M\nUID:02D84865-7611-4380-9B6E-41EAE7D4A6C8\nX-WR-ALARMUID:02D84865-7611-4380-9B6E-41EAE7D4A6C8\nEND:VALARM\nBEGIN:VALARM\nACTION:DISPLAY\nDESCRIPTION:Reminder\nTRIGGER:-P1D\nUID:087E2F5A-0104-4DAE-B6C7-E63A50A52B03\nX-WR-ALARMUID:087E2F5A-0104-4DAE-B6C7-E63A50A52B03\nEND:VALARM\nEND:VEVENT\nEND:VCALENDAR\n";
        let ical = r#"BEGIN:VCALENDAR
CALSCALE:GREGORIAN
PRODID:-//Apple Inc.//iOS 13.2//EN
VERSION:2.0
BEGIN:VTIMEZONE
TZID:America/Vancouver
BEGIN:DAYLIGHT
DTSTART:20070311T020000
RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=2SU
TZNAME:GMT-7
TZOFFSETFROM:-0800
TZOFFSETTO:-0700
END:DAYLIGHT
BEGIN:STANDARD
DTSTART:20071104T020000
RRULE:FREQ=YEARLY;BYMONTH=11;BYDAY=1SU
TZNAME:GMT-8
TZOFFSETFROM:-0700
TZOFFSETTO:-0800
END:STANDARD
END:VTIMEZONE
BEGIN:VEVENT
CREATED:20191002T181641Z
DTEND;TZID=America/Vancouver:20191014T090000
DTSTAMP:20191014T050818Z
DTSTART;TZID=America/Vancouver:20191014T070000
LAST-MODIFIED:20191002T181641Z
LOCATION:312 Boren Ave South\nSeattle WA 98144\nUSA
SEQUENCE:0
SUMMARY:Truline Swaybar Bushing Installation
TRANSP:OPAQUE
UID:2DC25870-241F-4279-9720-AB26C8DA5A60
URL;VALUE=URI:
X-APPLE-STRUCTURED-LOCATION;VALUE=URI;X-ADDRESS=312 Boren Ave South\\nSe
 attle WA 98144\\nUSA;X-APPLE-ABUID=Doug Tate Tru-Line’s Work;X-APPLE-MAP
 KIT-HANDLE=CAESjQIIwjsaEglN7OsKx8xHQBF/pyRGKpRewCKEAQoNVW5pdGVkIFN0YXRlc
 xICVVMaCldhc2hpbmd0b24iAldBKgRLaW5nMgdTZWF0dGxlOgU5ODE0NEIIQXRsYW50aWNSC
 0JvcmVuIEF2ZSBTWgMzMTJiDzMxMiBCb3JlbiBBdmUgU4oBEENlbnRyYWwgRGlzdHJpY3SKA
 QhBdGxhbnRpYyoPMzEyIEJvcmVuIEF2ZSBTMg8zMTIgQm9yZW4gQXZlIFMyElNlYXR0bGUsI
 FdBICA5ODE0NDINVW5pdGVkIFN0YXRlczg5QARaJAoiEhIJTezrCsfMR0ARf6ckRiqUXsAYw
 jsgodmfx+vg0YSuAQ==;X-APPLE-RADIUS=22.28845908357249;X-APPLE-REFERENCEFR
 AME=1;X-TITLE=312 Boren Ave South\\nSeattle WA 98144\\nUSA:geo:47.599824
 ,-122.315080
X-APPLE-TRAVEL-ADVISORY-BEHAVIOR:DISABLED
BEGIN:VALARM
ACTION:DISPLAY
DESCRIPTION:Reminder
TRIGGER:-PT30M
UID:02D84865-7611-4380-9B6E-41EAE7D4A6C8
X-WR-ALARMUID:02D84865-7611-4380-9B6E-41EAE7D4A6C8
END:VALARM
BEGIN:VALARM
ACTION:DISPLAY
DESCRIPTION:Reminder
TRIGGER:-P1D
UID:087E2F5A-0104-4DAE-B6C7-E63A50A52B03
X-WR-ALARMUID:087E2F5A-0104-4DAE-B6C7-E63A50A52B03
END:VALARM
END:VEVENT
END:VCALENDAR
"#;

        let parsed = Ical::parse(&LineIterator::new(ical));
        assert!(parsed.is_ok());
        let cal = parsed.unwrap();

        let apple_location = &cal.children[1].properties[11];
        assert_eq!(apple_location.name, "X-APPLE-STRUCTURED-LOCATION");
        assert_eq!(apple_location.value, "geo:47.599824,-122.315080");
    }
}
