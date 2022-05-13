use minicaldav::caldav::{
    get_calendars, get_events, get_home_set_url, get_principal_url, remove_event, save_event,
};
use ureq::Agent;
use url::Url;

const CALDAV_BASE_URL: &str = "localhost:8000";

const USER_PRINCIPAL_RESPONSE: &str = include_str!("responses/principal_response.xml");
const HOME_SET_RESPONSE: &str = include_str!("responses/home_set_response.xml");
const CALENDARS_RESPONSE: &str = include_str!("responses/calendars_response.xml");
const CALENDAR_ABC0815_RESPONSE: &str = include_str!("responses/calendar_ABC0815.xml");
const CALENDAR_ABC0816_RESPONSE: &str = include_str!("responses/calendar_ABC0816.xml");

mod mock {
    use super::*;
    use minicaldav::caldav::*;
    use once_cell::sync::Lazy;
    use std::{
        str::FromStr,
        sync::{
            mpsc::{channel, Sender},
            Arc, Mutex, MutexGuard,
        },
        thread::JoinHandle,
    };
    use tiny_http::{Header, Request, Response, Server};

    static MUTEX: Lazy<Mutex<()>> = Lazy::new(Mutex::default);

    pub struct MockServer {
        stop: Sender<()>,
        threads: Vec<JoinHandle<()>>,
        lock: MutexGuard<'static, ()>,
    }

    impl MockServer {
        pub fn end(self) {
            self.stop.send(()).unwrap();
            for t in self.threads {
                t.join().expect("Can not end mock server gracefully.");
            }
            drop(self.lock)
        }
    }

    pub fn mock_caldav_server() -> MockServer {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        let lock = MUTEX.lock().unwrap();
        let (send, recv) = channel();
        let server = Server::http(CALDAV_BASE_URL).unwrap();

        let server = Arc::new(server);
        let s = server.clone();
        let t1 = std::thread::spawn(move || {
            for mut request in s.incoming_requests() {
                println!("Got request {:?} {:?}", request.method(), request.url());
                if let Some(response) = mock_caldav_server_behaviour(&mut request) {
                    println!(
                        "Send response {:?} {:?}",
                        response.status_code(),
                        response.headers()
                    );
                    request.respond(response).unwrap();
                } else {
                    let message = format!(
                        "Request {:?} {:?} not mocked!",
                        request.method(),
                        request.url()
                    );
                    request
                        .respond(Response::from_string("Not mocked").with_status_code(400))
                        .unwrap();
                    println!("{}", message);
                    panic!("{}", message)
                }
            }
        });
        let t2 = std::thread::spawn(move || {
            if recv.recv().is_ok() {
                server.unblock();
            }
        });

        MockServer {
            stop: send,
            threads: vec![t1, t2],
            lock,
        }
    }

    fn mock_caldav_server_behaviour(
        request: &mut Request,
    ) -> Option<Response<std::io::Cursor<Vec<u8>>>> {
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap();

        if match_request(
            request,
            &body,
            "PROPFIND",
            "/",
            Some(USER_PRINCIPAL_REQUEST),
        ) {
            return Some(Response::from_string(USER_PRINCIPAL_RESPONSE));
        }
        if match_request(
            request,
            &body,
            "PROPFIND",
            "/principals/users/1",
            Some(HOMESET_REQUEST),
        ) {
            return Some(Response::from_string(HOME_SET_RESPONSE));
        }
        if match_request(
            request,
            &body,
            "PROPFIND",
            "/caldav/",
            Some(CALENDARS_REQUEST),
        ) {
            return Some(Response::from_string(CALENDARS_RESPONSE));
        }
        if match_request(
            request,
            &body,
            "REPORT",
            "/caldav/ABC0815/",
            Some(CALENDAR_EVENTS_REQUEST),
        ) {
            return Some(Response::from_string(CALENDAR_ABC0815_RESPONSE));
        }
        if match_request(
            request,
            &body,
            "REPORT",
            "/caldav/ABC0816/",
            Some(CALENDAR_EVENTS_REQUEST),
        ) {
            return Some(Response::from_string(CALENDAR_ABC0816_RESPONSE));
        }
        if match_request(request, &body, "PUT", "/caldav/ABC0815/1234", None) {
            let mut response = Response::from_string("");
            response.add_header(Header::from_str("ETag:1234-1").unwrap());
            return Some(response);
        }
        if match_request(request, &body, "DELETE", "/caldav/ABC0815/1234", None) {
            let mut response = Response::from_string("");
            response.add_header(Header::from_str("ETag:1234-1").unwrap());
            return Some(response);
        }
        None
    }

    fn match_request(
        request: &mut Request,
        body: &str,
        method: &str,
        url: &str,
        expected_body: Option<&str>,
    ) -> bool {
        request.method().as_str() == method
            && request.url() == url
            && (expected_body.is_none() || body == expected_body.unwrap())
    }
}

const USERNAME: &str = "foo";
const PASSWORD: &str = "bar";

fn get_client() -> Agent {
    ureq::AgentBuilder::new().build()
}

#[test]
pub fn test_get_user_principal() {
    let mockserver = mock::mock_caldav_server();
    let client = get_client();
    let base_url = Url::parse(&format!("http://{}", CALDAV_BASE_URL)).unwrap();
    let principal_url = get_principal_url(client, USERNAME, PASSWORD, &base_url)
        .expect("Failed to get principal url");
    assert_eq!(
        principal_url.as_str(),
        format!("http://{}{}", CALDAV_BASE_URL, "/principals/users/1")
    );
    mockserver.end();
}

#[test]
pub fn test_get_home_set() {
    let mockserver = mock::mock_caldav_server();
    let client = get_client();
    let base_url = Url::parse(&format!("http://{}", CALDAV_BASE_URL)).unwrap();
    let home_set_url = get_home_set_url(client, USERNAME, PASSWORD, &base_url)
        .expect("Failed to get home_set url");
    assert_eq!(
        home_set_url.as_str(),
        format!("http://{}{}", CALDAV_BASE_URL, "/caldav/")
    );
    mockserver.end();
}

#[test]
pub fn test_get_calendars() {
    let mockserver = mock::mock_caldav_server();
    let client = get_client();
    let base_url = Url::parse(&format!("http://{}", CALDAV_BASE_URL)).unwrap();
    let calendars =
        get_calendars(client, USERNAME, PASSWORD, &base_url).expect("Failed to get calendars");
    assert_eq!(calendars.len(), 2);
    assert_eq!(calendars.get(0).unwrap().name, "Calendar");
    assert_eq!(calendars.get(1).unwrap().name, "Birthdays");
    mockserver.end();
}

#[test]
pub fn test_get_events() {
    let mockserver = mock::mock_caldav_server();
    let client = get_client();
    let base_url = Url::parse(&format!("http://{}", CALDAV_BASE_URL)).unwrap();
    let calendars = get_calendars(client.clone(), USERNAME, PASSWORD, &base_url)
        .expect("Failed to get calendars");
    assert_eq!(calendars.len(), 2);
    assert_eq!(calendars.get(0).unwrap().name, "Calendar");
    assert_eq!(calendars.get(1).unwrap().name, "Birthdays");

    let events = get_events(
        client.clone(),
        USERNAME,
        PASSWORD,
        &base_url,
        calendars.get(0).unwrap(),
    )
    .expect("Failed to get events");

    let birthdays = get_events(
        client,
        USERNAME,
        PASSWORD,
        &base_url,
        calendars.get(1).unwrap(),
    )
    .expect("Failed to get birthdays");

    assert_eq!(events.len(), 4);
    assert_eq!(birthdays.len(), 1);

    assert!(events
        .get(0)
        .unwrap()
        .data
        .contains("SUMMARY:Event with timezone"));
    assert!(events
        .get(1)
        .unwrap()
        .data
        .contains("SUMMARY:Two day event with alarm"));
    assert!(events.get(2).unwrap().data.contains("SUMMARY:Yearly event"));
    assert!(events
        .get(3)
        .unwrap()
        .data
        .contains("SUMMARY:Miminal All-Day Event"));

    assert!(birthdays
        .get(0)
        .unwrap()
        .data
        .contains("SUMMARY:🎂 John Doe"));

    mockserver.end();
}

#[test]
pub fn test_save_events() {
    let mockserver = mock::mock_caldav_server();
    let client = get_client();
    let base_url = Url::parse(&format!("http://{}", CALDAV_BASE_URL)).unwrap();
    let calendars = get_calendars(client.clone(), USERNAME, PASSWORD, &base_url)
        .expect("Failed to get calendars");
    assert_eq!(calendars.len(), 2);
    assert_eq!(calendars.get(0).unwrap().name, "Calendar");
    assert_eq!(calendars.get(1).unwrap().name, "Birthdays");

    let calendar = &calendars.get(0).unwrap();
    let events = get_events(client.clone(), USERNAME, PASSWORD, &base_url, calendar)
        .expect("Failed to get events");

    assert_eq!(events.len(), 4);
    let mut event0 = events.get(0).unwrap().clone();
    event0.url = Url::parse(&format!("{}{}", calendar.url, "1234")).unwrap();
    event0.data = event0
        .data
        .replace("SUMMARY:Event with timezone", "SUMMARY:Event 1234");
    event0 = save_event(client, USERNAME, PASSWORD, event0).expect("Failed to create event");

    assert_eq!(event0.etag, Some("1234-1".into()));

    mockserver.end();
}

#[test]
pub fn test_delete_events() {
    let mockserver = mock::mock_caldav_server();
    let client = get_client();
    let base_url = Url::parse(&format!("http://{}", CALDAV_BASE_URL)).unwrap();
    let calendars = get_calendars(client.clone(), USERNAME, PASSWORD, &base_url)
        .expect("Failed to get calendars");
    assert_eq!(calendars.len(), 2);
    assert_eq!(calendars.get(0).unwrap().name, "Calendar");
    assert_eq!(calendars.get(1).unwrap().name, "Birthdays");

    let calendar = &calendars.get(0).unwrap();
    let events = get_events(client.clone(), USERNAME, PASSWORD, &base_url, calendar)
        .expect("Failed to get events");

    assert_eq!(events.len(), 4);
    let mut event0 = events.get(0).unwrap().clone();
    event0.url = Url::parse(&format!("{}{}", calendar.url, "1234")).unwrap();
    event0.data = event0
        .data
        .replace("SUMMARY:Event with timezone", "SUMMARY:Event 1234");
    remove_event(client, USERNAME, PASSWORD, event0).expect("Failed to create event");

    mockserver.end();
}
