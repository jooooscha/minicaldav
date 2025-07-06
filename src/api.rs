
use crate::vcalendar_props::VCalendarProps;
use crate::{caldav, calendar::Calendar};
use crate::errors::MiniCaldavError;
use icalendar::CalendarComponent;
use reqwest::Client;
use url::Url;
pub use icalendar::{
    CalendarComponent::*,
    Component as _,
    Property,
};
use log::*;

pub use crate::credentials::Credentials;
use crate::VCalendar;

/// Simple connection check to the DAV server
pub async fn check_connection(
    client: &Client,
    credentials: &Credentials,
    base_url: &Url,
) -> Result<Url, MiniCaldavError> {
    caldav::check_connetion(client, credentials, base_url).await
}

/// Get all calendars from the given CalDAV endpoint.
pub async fn get_calendars(
    client: &Client,
    credentials: &Credentials,
    base_url: Url,
) -> Result<Vec<Calendar>, MiniCaldavError> {
    let calendar_refs = caldav::get_calendars(client, credentials, base_url.clone()).await?;
    let mut calendars = Vec::new();
    for inner in calendar_refs {
        calendars.push(Calendar::new(base_url.clone(), inner));
    }
    Ok(calendars)
}

pub async fn create_calendar(
    client: &Client,
    credentials: &Credentials,
    base_url: &Url,
    calid: String,
    name: String,
    color: String,
) -> Result<(), MiniCaldavError> {
    caldav::create_calendar(client, credentials, base_url, calid, name, color).await?;
    Ok(())
}

pub async fn remove_calendar(
    client: &Client,
    credentials: &Credentials,
    base_url: &Url,
    calid: String,
) -> Result<(), MiniCaldavError> {
    caldav::remove_calendar(client, credentials, base_url, calid).await?;
    Ok(())
}

/// Get all events in the given `Calendar`.
/// This function returns a tuple of all events that could be parsed and all events that couldn't.
/// If anything besides parsing the event data fails, an Err will be returned.
pub async fn get_components(
    agent: &Client,
    credentials: &Credentials,
    calendar: &Calendar,
    start: Option<String>,
    end: Option<String>,
    expanded: bool,
) -> Result<Vec<VCalendarProps>, MiniCaldavError> {

    trace!("calendar.is_subscription(): {:?}", calendar.is_subscription());
    let responses = if calendar.is_subscription() {
        let export_url = Url::parse(&format!("{}?export", calendar.url())).unwrap();
        vec![caldav::get_ical_events(
            agent,
            credentials,
            export_url
        )
        .await?]
    } else {
        caldav::get_components(
            agent,
            credentials,
            calendar.base_url().clone(),
            calendar.url().clone(),
            start,
            end,
            expanded,
        )
        .await?
    };
    let mut events = Vec::new();

    for resp in responses {
        let parsed_calendar: VCalendar = resp.data().parse().map_err(|e| MiniCaldavError::ICalendar(e))?;
        trace!("parsed_calendar: {:#?}", parsed_calendar);
        let component = VCalendarProps::new(resp.url().clone(), resp.etag().clone(), parsed_calendar);
        events.push(component);
        // for comp in parsed_calendar.components {
        //     match comp {
        //         Event(event) => {
        //             let component = EventProp::new(resp.url().clone(), resp.etag().clone(), event);
        //             events.push(component);
        //         }
        //         Todo(_) => todo!(),
        //         Venue(_) => todo!(),
        //         _ => todo!(),
        //     }
        // }
    }
    trace!("Returning events: {:#?}", events);
    Ok(events)
}

/// Save the given event on the CalDAV server.
pub async fn save_component(
    client: &Client,
    credentials: &Credentials,
    mut vcalendar_props: VCalendarProps,
) -> Result<VCalendarProps, MiniCaldavError> {

    // update sequence number for each VEVENT
    for component in vcalendar_props.inner_mut().components.iter_mut() {
        match component {
            CalendarComponent::Event(event) => {
                let sequenc_old = event.properties().get("SEQUENCE").and_then(|prop| prop.get_value_as(|v| v.parse::<i64>().ok()));
                if let Some(num) = sequenc_old {
                    let sequenc_new = format!("{}", num + 1);
                    event.append_property(Property::new("SEQUENCE", &sequenc_new));
                }
            }
            _ => {
                warn!("Currently only Events are supported");
            }
        }
    }

    let new_component = caldav::save_component(client, credentials, vcalendar_props).await?;

    Ok(new_component)

}

/// Remove the given event on the CalDAV server.
pub async fn remove_component(
    client: &Client,
    credentials: &Credentials,
    url: Url,
) -> Result<(), MiniCaldavError> {
    caldav::remove_component(client, credentials, url).await?;
    Ok(())
}


// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct Property {
//     name: String,
//     value: String,
//     attributes: HashMap<String, String>,
// }

// impl Property {
//     pub fn new(name: &str, value: &str) -> Self {
//         Self {
//             name: name.into(),
//             value: value.into(),
//             attributes: Default::default(),
//         }
//     }

//     pub fn new_with_attributes(name: &str, value: &str, attribs: Vec<(&str, &str)>) -> Self {
//         let mut attributes = HashMap::new();
//         for (k, v) in attribs {
//             attributes.insert(k.into(), v.into());
//         }
//         Self {
//             name: name.into(),
//             value: value.into(),
//             attributes,
//         }
//     }

//     pub fn name(&self) -> &String {
//         &self.name
//     }

//     pub fn value(&self) -> &String {
//         &self.value
//     }

//     pub fn into_value(self) -> String {
//         self.value
//     }

//     pub fn attribute(&self, name: &str) -> Option<&String> {
//         self.attributes.get(name)
//     }
// }

// impl From<ical::Property> for Property {
//     fn from(p: ical::Property) -> Self {
//         Self {
//             name: p.name,
//             value: p.value,
//             attributes: p.attributes,
//         }
//     }
// }

// impl From<Property> for ical::Property {
//     fn from(p: Property) -> Self {
//         Self {
//             name: p.name,
//             value: p.value,
//             attributes: p.attributes,
//         }
//     }
// }

// #[derive(Debug)]
// pub struct InstanceBuilder {
//     url: Url,
//     etag: Option<String>,
//     ical: CalendarComponent,
// }

// impl InstanceBuilder {
//     pub fn build_event(self) -> Instance {
//         Instance {
//             etag: self.etag,
//             url: self.url,
//             ical: self.ical,
//         }
//     }

//     pub fn build_todo(self) -> Instance {
//         todo!("TODOs are not yet supported");
//     }

//     pub fn etag(mut self, etag: Option<String>) -> Self {
//         self.etag = etag;
//         self
//     }

//     pub fn uid(mut self, value: String) -> Self {
//         let prop = Property::new("UID", value);

//         // self.properties.insert("UID", prop);
//         // self.properties.push(ical::Property {
//         //     name: "UID".to_string(),
//         //     value,
//         //     attributes: HashMap::new(),
//         // });
//         // self
//         todo!();
//     }

//     pub fn timestamp(mut self, value: String) -> Self {
//         todo!();
//         // self.properties.push(ical::Property {
//         //     name: "DTSTAMP".to_string(),
//         //     value,
//         //     attributes: HashMap::new(),
//         // });
//         // self
//     }

//     pub fn summary(mut self, value: String) -> Self {
//         todo!();
//         // self.properties.push(ical::Property {
//         //     name: "SUMMARY".to_string(),
//         //     value,
//         //     attributes: HashMap::new(),
//         // });
//         // self
//     }

//     pub fn priority(mut self, value: String) -> Self {
//         todo!();
//         // self.properties.push(ical::Property {
//         //     name: "PRIORITY".to_string(),
//         //     value,
//         //     attributes: HashMap::new(),
//         // });
//         // self
//     }

//     pub fn duedate(mut self, value: String) -> Self {
//         todo!();
//         // self.properties.push(ical::Property {
//         //     name: "DUE".to_string(),
//         //     value,
//         //     attributes: HashMap::new(),
//         // });
//         // self
//     }

//     pub fn status(mut self, value: String) -> Self {
//         todo!();
//         self.properties.push(ical::Property {
//             name: "STATUS".to_string(),
//             value,
//             attributes: HashMap::new(),
//         });
//         self
//     }

//     pub fn generic(mut self, name: String, value: String) -> Self {
//         self.properties.push(ical::Property {
//             name,
//             value,
//             attributes: HashMap::new(),
//         });
//         self
//     }

//     pub fn location(mut self, value: Option<String>) -> Self {
//         if let Some(value) = value {
//             self.properties.push(ical::Property {
//                 name: "LOCATION".to_string(),
//                 value,
//                 attributes: HashMap::new(),
//             });
//         }
//         self
//     }

//     pub fn start(mut self, value: String, attributes: Vec<(&str, &str)>) -> Self {
//         let mut attribs = HashMap::new();
//         for (k, v) in attributes {
//             attribs.insert(k.into(), v.into());
//         }
//         self.properties.push(ical::Property {
//             name: "DTSTART".to_string(),
//             value,
//             attributes: attribs,
//         });
//         self
//     }

//     pub fn end(mut self, value: String, attributes: Vec<(&str, &str)>) -> Self {
//         let mut attribs = HashMap::new();
//         for (k, v) in attributes {
//             attribs.insert(k.into(), v.into());
//         }
//         self.properties.push(ical::Property {
//             name: "DTEND".to_string(),
//             value,
//             attributes: attribs,
//         });
//         self
//     }

//     pub fn description(mut self, value: Option<String>) -> Self {
//         if let Some(value) = value {
//             self.properties.push(ical::Property {
//                 name: "DESCRIPTION".to_string(),
//                 value,
//                 attributes: HashMap::new(),
//             });
//         }
//         self
//     }

//     pub fn rrule(mut self, value: Option<String>) -> Self {
//         if let Some(value) = value {
//             self.properties.push(ical::Property {
//                 name: "RRULE".to_string(),
//                 value,
//                 attributes: HashMap::new(),
//             });
//         }
//         self
//     }
// }
