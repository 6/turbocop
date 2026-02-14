Time.zone = "Eastern"
^^^^^^^^^^^^^^^^^^^^^ Rails/TimeZoneAssignment: Do not set `Time.zone` directly. Use `Time.use_zone` instead.

Time.zone = "UTC"
^^^^^^^^^^^^^^^^^ Rails/TimeZoneAssignment: Do not set `Time.zone` directly. Use `Time.use_zone` instead.

Time.zone = user.time_zone
^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/TimeZoneAssignment: Do not set `Time.zone` directly. Use `Time.use_zone` instead.