foo.bar && foo&.baz
              ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo&.bar && foo&.baz
               ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo.bar || foo&.baz
              ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
