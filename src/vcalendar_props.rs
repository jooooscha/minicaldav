use icalendar::CalendarComponent;
use icalendar::Other;
use log::warn;
use url::Url;
use crate::VCalendar;
use crate::VEvent;
use icalendar::Component;

/// An event in a CalDAV calendar.
/// Corresponds to exactly one `.ics` file
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VCalendarProps {
    url: Url,
    etag: Option<String>,
    inner: VCalendar,
}

impl VCalendarProps {
    /// Construct a new event.
    pub fn new(url: Url, etag: Option<String>, inner: VCalendar) -> Self {
        Self { url, etag, inner }
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn into_url(self) -> Url {
        self.url
    }

    pub fn set_url(&mut self, url: Url) {
        self.url = url;
    }

    pub fn etag(&self) -> Option<&String> {
        self.etag.as_ref()
    }

    pub fn set_etag(&mut self, etag: Option<String>) {
        self.etag = etag;
    }

    pub fn inner(&self) -> &VCalendar {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut VCalendar {
        &mut self.inner
    }

    pub fn into_inner(self) -> VCalendar {
        self.inner
    }

    pub fn set_inner(&mut self, component: VCalendar) {
        self.inner = component;
    }

    /// Returns `true` if any of the VEvents in this VCalendar have the UID `uid`
    pub fn contains_uid(&self, uid: &String) -> bool {
        self.inner().components.iter().any(|component|
            component.as_todo().map(|event|
                event.properties().get("UID").map(|event_uid|
                    event_uid.value() == uid
                )
                .unwrap_or(false) // if the UID does not match
            )
            .unwrap_or(false) // if event is not an VEVENT
        )
    }

    /// Returns the first VEvent that matches `uid`
    pub fn get_vevent(&self, uid: &String) -> Option<&VEvent>{
        self.inner()
            .components.iter()
            .find_map(|vevent: &CalendarComponent| {
                match vevent {
                    CalendarComponent::Event(event) => {
                        // select event if it has correct UID
                        let event_uid = event
                            .properties()
                            .get("UID")
                            .map(|uid_prop| uid_prop.value());

                        if event_uid == Some(&uid) {
                            Some(event)
                        } else {
                            None
                        }
                    },
                    _ => {
                        warn!("Currently only Events are supported");
                        None
                    }
                }
            })
    }

    /// Returns a mutable reference to the first VEvent that matches `uid`
    pub fn get_vevent_mut(&mut self, uid: &String) -> Option<&mut VEvent>{
        self.inner_mut()
            .components.iter_mut()
            .find_map(|vevent: &mut CalendarComponent| {
                match vevent {
                    CalendarComponent::Event(event) => {
                        // select event if it has correct UID
                        let event_uid = event
                            .properties()
                            .get("UID")
                            .map(|uid_prop| uid_prop.value());

                        if event_uid == Some(&uid) {
                            Some(event)
                        } else {
                            None
                        }
                    },
                    _ => {
                        warn!("Currently only Events are supported");
                        None
                    }
                }
            })
    }

    /// Returns the first VEvent with matching RECURRENCE-ID
    pub fn get_vevent_by_recurence_id(&self, recurrence_id: &String) -> Option<&VEvent> {
        self.inner()
            .components.iter()
            .find_map(|vevent: &CalendarComponent| {
                match vevent {
                    CalendarComponent::Event(event) => {
                        // select event if it has correct UID
                        let event_uid = event
                            .properties()
                            .get("RECURRENCE-ID")
                            .map(|uid_prop| uid_prop.value());

                        if event_uid == Some(&recurrence_id) {
                            Some(event)
                        } else {
                            None
                        }
                    },
                    _ => {
                        warn!("Currently only Events are supported");
                        None
                    }
                }
            })
    }

    /// Returns a mutable reference to the first VEvent with matching RECURRENCE-ID
    pub fn get_vevent_by_recurence_id_mut(&mut self, recurrence_id: &String) -> Option<&mut VEvent> {
        self.inner_mut()
            .components.iter_mut()
            .find_map(|vevent: &mut CalendarComponent| {
                match vevent {
                    CalendarComponent::Event(event) => {
                        // select event if it has correct UID
                        let event_uid = event
                            .properties()
                            .get("RECURRENCE-ID")
                            .map(|uid_prop| uid_prop.value());

                        if event_uid == Some(&recurrence_id) {
                            Some(event)
                        } else {
                            None
                        }
                    },
                    _ => {
                        warn!("Currently only Events are supported");
                        None
                    }
                }
            })
    }

    // fn get_property(&self, name: &str) -> Option<&Property> {
    //     self.ical.properties().get(name)
    //     // self.ical.properties().get(datatype).and_then(|ical| {
    //     //     prop.iter().find_map(|p| {
    //     //         if p.name == name {
    //     //             Some(Property::from(p.clone()))
    //     //         } else {
    //     //             None
    //     //         }
    //     //     })
    //     // })
    // }

    // /// Get the property of the given name or `None`.
    // pub fn pop_property(&mut self, name: &str) -> Option<Property> {
    //     self.ical.properties.get_mut("VEVENT").and_then(|ical| {
    //         let index = ical.properties.iter().enumerate().find_map(|(i, p)| {
    //             if p.name == name {
    //                 Some(i)
    //             } else {
    //                 None
    //             }
    //         });

    //         if let Some(index) = index {
    //             Some(Property::from(ical.properties.remove(index)))
    //         } else {
    //             None
    //         }
    //     })
    // }

    // pub fn ical(&self) -> &CalendarComponent {
    //     &self.ical
    // }

    // pub fn ical_mut(&mut self) -> &mut CalendarComponent {
    //     &mut self.ical
    // }

    // pub fn add(&mut self, property: Property) {
    //     if let Some(ical) = self.ical.get_mut("VEVENT") {
    //         ical.properties.push(property.into());
    //     }
    // }

    // pub fn property(&self, name: &str) -> Option<Property> {
    //     self.get_property(name, "VEVENT")
    // }

    // /// Get the property of the given name or `None`.
    // pub fn property_todo(&self, name: &str) -> Option<Property> {
    //     self.get_property(name, "VTODO")
    // }

    // /// Get the value of the given property name or `None`.
    // pub fn get(&self, name: &str) -> Option<&String> {
    //     self.ical.get("VEVENT").and_then(|ical| {
    //         ical.properties
    //             .iter()
    //             .find_map(|p| if p.name == name { Some(&p.value) } else { None })
    //     })
    // }

    // /// Get the value of the given property name or `None`.
    // pub fn get_all(&self, name: &str) -> Option<Vec<&String>> {
    //     self.ical.get("VEVENT").map(|ical| {
    //         ical.properties
    //             .iter()
    //             .filter_map(|p| if p.name == name { Some(&p.value) } else { None })
    //             .collect::<Vec<&String>>()
    //     })
    // }

    // /// Set the value of the given property name or create a new property.
    // pub fn set(&mut self, name: &str, value: &str) {
    //     match self
    //         .ical
    //         .get_mut("VEVENT")
    //         .and_then(|e| e.properties.iter_mut().find(|p| p.name == name))
    //     {
    //         Some(p) => {
    //             p.value = value.into();
    //         }
    //         None => self.add(Property::new(name, value)),
    //     }
    // }

    // pub fn set_property_attribute(&mut self, name: &str, attr_name: &str, attr_value: &str) {
    //     if let Some(p) = self
    //         .ical
    //         .get_mut("VEVENT")
    //         .and_then(|e| e.properties.iter_mut().find(|p| p.name == name))
    //     {
    //         p.attributes.insert(attr_name.into(), attr_value.into());
    //     }
    // }

    // pub fn remove_property_attribute(&mut self, name: &str, attr_name: &str) {
    //     if let Some(p) = self
    //         .ical
    //         .get_mut("VEVENT")
    //         .and_then(|e| e.properties.iter_mut().find(|p| p.name == name))
    //     {
    //         p.attributes.remove(attr_name);
    //     }
    // }

    // /// Get all properties of this event.
    // fn get_properties(&self, datatype: &str) -> Vec<(&String, &String)> {
    //     self.ical
    //         .get(datatype)
    //         .map(|ical| {
    //             ical.properties
    //                 .iter()
    //                 .map(|p| (&p.name, &p.value))
    //                 .collect::<Vec<(&String, &String)>>()
    //         })
    //         .unwrap_or_else(|| {
    //             error!("Could not get properties: No VEVENT section.");
    //             Vec::new()
    //         })
    // }

    // pub fn into_properties(self) -> Vec<Property> {
    //     for ical in self.ical.children {
    //         if ical.name == "VEVENT" {
    //             return ical.properties.into_iter().map(Property::from).collect();
    //         }
    //     }
    //     Vec::new()
    // }

    // /// Get all properties of this event.
    // pub fn properties(&self) -> Vec<(&String, &String)> {
    //     self.get_properties("VEVENT")
    // }

    // /// Get all properties of this todo.
    // pub fn properties_todo(&self) -> Vec<(&String, &String)> {
    //     self.get_properties("VTODO")
    // }

    // pub fn add_component(&mut self, ical: Ical) {
    //     self.ical_mut().add_component(ical);
    // }

    // pub fn get_first_component_mut(&mut self) -> Option<&mut Ical> {
    //     self.ical.children.first_mut()
    // }

    // pub fn get_component_by_recurid(&mut self, recurid: &str) -> Option<&mut Ical> {
    //     // search for the first component with name RECURRENCE-ID
    //     // where the value matches `recurid`
    //     // In other words, return the component with matching RECURRENCE-ID
    //     // if it exists
    //     self.ical.children.iter_mut().find(|comp| {
    //         comp.get_first_property("RECURRENCE-ID")
    //             .map(|p| p.value == recurid)
    //             .unwrap_or(false)
    //     })
    // }
}

/// Holds relevant information like `etag` and `url` and the not-yet parsed `icalendar::Calendar` data
#[derive(Clone, Debug)]
pub struct ComponentResponse {
    url: Url,
    etag: Option<String>,
    data: String,
}

impl ComponentResponse {
    pub fn new<S: Into<String>>(url: Url, etag: Option<String>, data: S) -> Self {
        Self { url, etag, data: data.into() }
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn etag(&self) -> &Option<String> {
        &self.etag
    }

    pub fn data(&self) -> &String {
        &self.data
    }
}
