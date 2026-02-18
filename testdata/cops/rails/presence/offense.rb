a.present? ? a : nil
^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `a.presence` instead of `a.present? ? a : nil`.
a.blank? ? nil : a
^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `a.presence` instead of `a.blank? ? nil : a`.
a.present? ? a : b
^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `a.presence || b` instead of `a.present? ? a : b`.
!a.present? ? nil : a
^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `a.presence` instead of `!a.present? ? nil : a`.
!a.blank? ? a : nil
^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `a.presence` instead of `!a.blank? ? a : nil`.
