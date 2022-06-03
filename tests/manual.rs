use minicaldav::caldav::get_calendars;
use url::Url;

const URL: &str = "https://...";

// #[test]
pub fn test_get_calendars_without_homeset() {
    let client = ureq::AgentBuilder::new().build();
    let base_url = Url::parse(URL).unwrap();
    let calendars = get_calendars(client, "", "", &base_url).expect("Failed to get calendars");
    println!("{:?}", calendars)
}
