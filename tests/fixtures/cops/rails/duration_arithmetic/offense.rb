Time.current + 1.day
^^^^^^^^^^^^^^^^^^^^ Rails/DurationArithmetic: Do not add or subtract duration.
Time.current - 2.hours
^^^^^^^^^^^^^^^^^^^^^^ Rails/DurationArithmetic: Do not add or subtract duration.
Time.zone.now + 30.minutes
^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DurationArithmetic: Do not add or subtract duration.
::Time.zone.now + 1.hour
^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DurationArithmetic: Do not add or subtract duration.
