use icalendar::Event;
use url::Url;

/// An event in a CalDAV calendar.
/// Corresponds to exactly one `.ics` file
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Component {
    url: Url,
    etag: Option<String>,
    inner: Event,
}

impl Component {
    /// Construct a new event.
    pub fn new(url: Url, etag: Option<String>, inner: Event) -> Self {
        Self { url, etag, inner }
    }

    pub fn url(&self) -> &Url {
        &self.url
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

    pub fn inner(&self) -> &Event {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut Event {
        &mut self.inner
    }

    pub fn into_inner(self) -> Event {
        self.inner
    }

    pub fn set_inner(&mut self, component: Event) {
        self.inner = component;
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
