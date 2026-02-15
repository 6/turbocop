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
