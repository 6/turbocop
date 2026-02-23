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
field.destroy if field.present?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `field.presence&.destroy` instead of `field.destroy if field.present?`.
reply_to_post.present? ? reply_to_post.post_number : nil
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `reply_to_post.presence&.post_number` instead of `reply_to_post.present? ? reply_to_post.post_number : nil`.
!a.blank? ? a.foo : nil
^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `a.presence&.foo` instead of `!a.blank? ? a.foo : nil`.
