pub static USER_PRINCIPAL_REQUEST: &str = r#"
    <d:propfind xmlns:d="DAV:">
       <d:prop>
           <d:current-user-principal />
       </d:prop>
    </d:propfind>
"#;

pub static HOMESET_REQUEST: &str = r#"
    <d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" >
      <d:self/>
      <d:prop>
        <c:calendar-home-set />
      </d:prop>
    </d:propfind>
"#;

pub static CALENDARS_REQUEST: &str = r#"
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" >
    <d:prop>
        <d:displayname />
        <d:resourcetype />
        <d:current-user-privilege-set/>
        <calendar-color xmlns="http://apple.com/ns/ical/" />
        <c:supported-calendar-component-set />
    </d:prop>
</d:propfind>
"#;

// pub static CALENDAR_TODOS_REQUEST: &str = r#"
//     <c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
//         <d:prop>
//             <d:getetag />
//             <c:calendar-data />
//         </d:prop>
//         <c:filter>
//             <c:comp-filter name="VCALENDAR">
//                 <c:comp-filter name="VTODO" />
//             </c:comp-filter>
//         </c:filter>
//     </c:calendar-query>
// "#;

pub static CALENDARS_QUERY: &str = r#"
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
    <d:prop>
        <d:getetag />
        <d:displayname />
        <d:current-user-privilege-set/>
        <calendar-color xmlns="http://apple.com/ns/ical/" />
        <d:resourcetype />
        <c:supported-calendar-component-set />
    </d:prop>
    <c:filter>
        <c:comp-filter name="VCALENDAR" />
    </c:filter>
</c:calendar-query>
"#;

pub static CALENDAR_EVENTS_REQUEST: &str = r#"
    <c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
        <d:prop>
            <d:getetag />
            <c:calendar-data />
        </d:prop>
        <c:filter>
            <c:comp-filter name="VCALENDAR">
                <c:comp-filter name="VEVENT" />
            </c:comp-filter>
        </c:filter>
    </c:calendar-query>
"#;
// <c:calendar-data>
//     <c:expand start="20000103T000000Z" end="21000105T000000Z"/>
// </c:calendar-data>
//<c:comp-filter name="VEVENT">
//    <c:time-range start="20250103T000000Z" end="20260105T000000Z"/>
//</c:comp-filter>

pub fn build_create_calendar_xml(name: String, color: String) -> String {
    format!(
        r#"
    <x0:mkcol xmlns:x0="DAV:">
	<x0:set>
		<x0:prop>
			<x0:resourcetype>
				<x0:collection/>
				<x1:calendar xmlns:x1="urn:ietf:params:xml:ns:caldav"/>
				</x0:resourcetype>
				<x0:displayname>{name}</x0:displayname>
				<x6:calendar-color
					xmlns:x6="http://apple.com/ns/ical/">{color}
				</x6:calendar-color>
				<x4:calendar-enabled
					xmlns:x4="http://owncloud.org/ns">1
				</x4:calendar-enabled>
				<x1:calendar-timezone
					xmlns:x1="urn:ietf:params:xml:ns:caldav">BEGIN:VCALENDAR
PRODID:-//IDN nextcloud.com//Calendar app 5.2.2//EN
CALSCALE:GREGORIAN
VERSION:2.0
BEGIN:VTIMEZONE
TZID:Europe/Berlin
BEGIN:DAYLIGHT
TZOFFSETFROM:+0100
TZOFFSETTO:+0200
TZNAME:CEST
DTSTART:19700329T020000
RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU
END:DAYLIGHT
BEGIN:STANDARD
TZOFFSETFROM:+0200
TZOFFSETTO:+0100
TZNAME:CET
DTSTART:19701025T030000
RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU
END:STANDARD
END:VTIMEZONE
END:VCALENDAR
				</x1:calendar-timezone>
				<x1:supported-calendar-component-set
					xmlns:x1="urn:ietf:params:xml:ns:caldav">
					<x1:comp name="VEVENT"/>
				</x1:supported-calendar-component-set>
			</x0:prop>
		</x0:set>
	</x0:mkcol>
        "#
    )
}

pub fn build_calendar_request_string(
    start: Option<String>,
    end: Option<String>,
    expanded: bool,
) -> String {
    let start = start.as_deref().unwrap_or("20000103T000000Z");
    let end = end.as_deref().unwrap_or("21000105T000000Z");

    let req = if expanded {
        format!(
            r#"<c:calendar-data>
          <c:expand start="{}" end="{}"/>
       </c:calendar-data>"#,
            start, end
        )
    } else {
        format!(
            r#"<c:calendar-data>
          <c:limit-recurrance-set start="{}" end="{}"/>
       </c:calendar-data>"#,
            start, end
        )
    };

    let range = format!(
        r#"<c:comp-filter name="VEVENT">
                   <c:time-range start="{}" end="{}"/>
               </c:comp-filter>"#,
        start, end
    );

    format!(
        r#"
    <c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
        <d:prop>
            <d:getetag />
            {}
        </d:prop>
        <c:filter>
            <c:comp-filter name="VCALENDAR">
                {}
            </c:comp-filter>
        </c:filter>
    </c:calendar-query>
   "#,
        req, range
    )
}
