Const&.do_something
     ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

self&.foo
    ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

foo.to_s&.strip
        ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

42&.minutes
  ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.

'hello'&.upcase
       ^^ Lint/RedundantSafeNavigation: Redundant safe navigation detected, use `.` instead.
