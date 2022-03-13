# minicaldav

Small and easy CalDAV client.

minicaldav is a caldav client and basic ical parser with as little dependencies as possible (but practical).

## Project scope:
- Simple: Few dependencies, no async, the code is simple
- Correct: CalDAV and ICAL are implemented correctly
- Tested: CalDAV works with Events, and is tested with multiple common services
- Easy to use: Good documentation and intuitive API

# Project status
- minicaldav is neither correct nor sufficient tested at this point
    - Tested only with one caldav server
- minicaldav is in activley maintained
- minicaldav is used in a public app: https://gitlab.com/loers/karlender

# Quick Start

```rust
use url::Url;
use ureq::Agent;
pub fn main() {
    let agent = Agent::new();
    let url = Url::parse("http://mycaldav.com/").unwrap();
    let username = "foo";
    let password = "s3cret!";
    let calendars = minicaldav::get_calendars(agent.clone(), username, password, &url).unwrap();
    for calendar in calendars {
        println!("{:?}", calendar);
        let events = minicaldav::get_events(agent.clone(), username, password, &calendar).unwrap();
        for event in events {
            println!("{:?}", event);
        }
    }
}
```
