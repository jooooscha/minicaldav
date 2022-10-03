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

//! Simple CLI tool mostly for testing purposes currently.

#[cfg(feature = "cli")]
#[macro_use]
extern crate log;

#[cfg(feature = "cli")]
mod api;
#[cfg(feature = "cli")]
mod caldav;
#[cfg(feature = "cli")]
mod ical;

#[cfg(not(feature = "cli"))]
pub fn main() {}

#[cfg(feature = "cli")]
pub fn main() {
    env_logger::init();

    use ureq::Agent;
    use url::Url;

    #[rustfmt::skip]
    const FUNCTIONS: [(&str, &str); 2] = [
        ("get_calendars                        ", "Get a list of calendars without events",),
        ("get_events    <Name of the calendar> ", "Get a list of all events in the given calendar."),
    ];

    fn help() {
        let mut functions = String::new();
        for f in FUNCTIONS {
            functions.push_str(&format!("- {}: {}\n", f.0, f.1));
        }
        println!("Use either one of:\n{}", functions);
    }

    fn login() -> (String, String, String) {
        let url = if let Ok(url) = std::env::var("URL") {
            url
        } else {
            read("Enter caldav url")
        };
        let email = if let Ok(email) = std::env::var("EMAIL") {
            email
        } else {
            read("Enter email")
        };
        println!("Enter password");
        let password = rpassword::read_password().unwrap();
        (url, email, password)
    }

    fn read(message: &str) -> String {
        println!("{}", message);
        let mut buffer = String::new();
        let stdin = std::io::stdin();
        stdin.read_line(&mut buffer).unwrap();
        buffer
    }

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        help();
        return;
    }

    let fun = args.get(1).unwrap();
    let agent = Agent::new();
    match fun.as_str() {
        "get_calendars" => {
            let (url, user, pswd) = login();
            let calendars =
                api::get_calendars(agent, &user, &pswd, &Url::parse(&url).unwrap()).unwrap();
            for calendar in calendars {
                println!("{} {}", calendar.name(), calendar.url().as_str());
            }
        }
        "get_events" => {
            let (url, user, pswd) = login();
            let name = if args.len() >= 3 {
                args.get(2).unwrap().clone()
            } else {
                read("Calendar name:")
            };
            let calendars =
                api::get_calendars(agent.clone(), &user, &pswd, &Url::parse(&url).unwrap())
                    .unwrap();

            for calendar in calendars {
                if calendar.name() == &name {
                    let events = api::get_events(agent.clone(), &user, &pswd, &calendar).unwrap();
                    for event in events.0 {
                        for (k, v) in event.properties() {
                            println!("{}: {}", k, v);
                        }
                        println!("--------------------------------------------");
                    }
                }
            }
        }
        _ => help(),
    }
}
