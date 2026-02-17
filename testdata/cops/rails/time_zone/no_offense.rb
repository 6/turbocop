Time.current
Time.zone.now
foo.now
DateTime.current
Process.clock_gettime(Process::CLOCK_MONOTONIC)
Time.now.utc
Time.now.in_time_zone
Time.now.getutc
Time.now.to_i
Time.utc(2000)
Time.gm(2000, 1, 1)
I18n.l(Time.now.utc)
foo(bar: Time.now.in_time_zone)
# String argument with timezone specifier — RuboCop skips these
Time.parse('2023-05-29 00:00:00 UTC')
Time.parse('2015-03-02T19:05:37Z')
Time.parse('2015-03-02T19:05:37+05:00')
Time.parse('2015-03-02T19:05:37-0500')
