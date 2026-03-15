Time.current
Time.zone.now
foo.now
DateTime.current
Process.clock_gettime(Process::CLOCK_MONOTONIC)
Time.now.utc
Time.now.in_time_zone
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
# Time.at/new/now with `in:` keyword argument — timezone offset provided
Time.at(epoch, in: "UTC")
Time.now(in: "+09:00")
Time.new(2023, 1, 1, in: "UTC")
# Method chains with intermediate calls before timezone-safe method
Time.at(timestamp).to_datetime.in_time_zone
Time.at(payload.updated_at / 1000).to_datetime.in_time_zone("UTC")
Time.now.to_i
Time.parse(str).iso8601
# Qualified constant paths — NOT top-level Time, should not be flagged
Some::Time.now
Module::Time.parse("2023-01-01")
Foo::Bar::Time.at(0)
Some::Time.new(2023, 1, 1)
Some::Time.local(2023, 1, 1)
Some::Time.now(0).strftime('%H:%M')

# Time.parse with interpolated string ending in timezone specifier
Time.parse("#{ts} UTC")
Time.parse("#{string}Z", true)
Time.parse("#{val} +05:00")

# Time.now/local inside arguments of a safe method (RuboCop parent-chain walk)
Time.utc(Time.now.year - 1, 7, 1, 0, 0, 0)
Time.utc(Time.now.year, 1, 1)

# Time.now inside Time.at(..., in:) — parent provides timezone context
Time.at(Time.now, in: 'UTC')
Time.at(Time.now, in: 'Z')
Time.at(Time.now, in: '-00:00')

# .localtime WITH arguments is safe
Time.now.localtime("+09:00")
Time.at(time).localtime("+05:30")
