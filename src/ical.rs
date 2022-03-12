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
        let mut ical = None;
        while let Some(line) = lines.next() {
            if line.trim().is_empty() {
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
                ical.as_mut().unwrap().children.push(child);
                continue;
            }
            if let Some(name) = prop.is("END") {
                if name == &ical.as_ref().unwrap().name {
                    return Ok(ical.unwrap());
                }
            }
            ical.as_mut().unwrap().properties.push(prop);
        }
        if ical.is_some() {
            Err(Error::new(format!("Missing END:{}", ical.unwrap().name)))
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
        if let Some((name, value)) = input.split_once(':') {
            let mut parts = name.split(';');
            let mut property = Property::new(parts.next().unwrap().trim(), value);
            for part in parts {
                if let Some((k, v)) = part.split_once('=') {
                    property.attributes.insert(k.into(), v.into());
                }
            }
            Ok(property)
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
        format!("{}:{}", self.name, self.value)
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
        let ical = r#"
        BEGIN:VCALENDAR
        VERSION:2.0
        PRODID:-//ZContent.net//Zap Calendar 1.0//EN
        CALSCALE:GREGORIAN
        METHOD:PUBLISH
        END:VCALENDAR
        "#;
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
        let ical = r#"
        BEGIN:VCALENDAR
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
