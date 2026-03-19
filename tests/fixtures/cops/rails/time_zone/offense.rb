# String#to_time without timezone specifier — bad in flexible mode (default)
"2012-03-02 16:05:37".to_time
                      ^^^^^^^ Rails/TimeZone: Do not use `String#to_time` without zone. Use `Time.zone.parse` instead.

"2005-02-27 23:50".to_time
                   ^^^^^^^ Rails/TimeZone: Do not use `String#to_time` without zone. Use `Time.zone.parse` instead.

Time.now
     ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

x = Time.now
         ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

if Time.now > deadline
        ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.
  puts "expired"
end

::Time.now
       ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

Time.now.getutc
     ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

# .localtime without arguments is NOT safe — RuboCop flags MSG_LOCALTIME
Time.at(time).localtime
     ^^ Rails/TimeZone: Use `Time.zone.at` instead of `Time.at`.

Time.at(@time).localtime.to_s
     ^^ Rails/TimeZone: Use `Time.zone.at` instead of `Time.at`.

Time.now.localtime
     ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

# Grouping parentheses — NOT method call parens, should still flag
(Time.now - 1.day).to_i
      ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

(first_seen_at || Time.now).to_i.to_s
                       ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

(Time.now - 7200).to_i
      ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

(Time.now - seconds).to_i
      ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

return (Time.now - 1.day).to_i if expired?
             ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

# utc? is a query method, NOT the same as .utc — should still flag
Time.now.utc?
     ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

Time.at(x).utc?
     ^^ Rails/TimeZone: Use `Time.zone.at` instead of `Time.at`.

Time.now.gmtime.utc?
     ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

# Time.now inside Time.at(..., in:) — the in: makes the OUTER call safe,
# but the inner Time.now still needs timezone awareness
Time.at(Time.now, in: 'UTC')
             ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

Time.at(Time.now, in: 'Z')
             ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

Time.at(Time.now, in: '-00:00')
             ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

# Grouping parens with space between method name and ( — the .to_f is on the
# grouped expression, NOT the enclosing call, so should still flag
schedule (Time.now - 60).to_f, arg2
               ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

expect(val).to eq (Time.now + 7.days).to_i
                        ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

# Safe navigation &. breaks the chain — csend is not send_type? in RuboCop
Time.at(val)&.utc
     ^^ Rails/TimeZone: Use `Time.zone.at` instead of `Time.at`.

Time.now&.to_i
     ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.
