pub fn build_create_calendar_xml(
    name: String,
    color: String
) -> String {
    format!(r#"
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
        "#)
}
