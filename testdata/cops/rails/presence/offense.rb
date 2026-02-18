a.present? ? a : nil
^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `a.presence` instead of `a.present? ? a : nil`.
a.blank? ? nil : a
^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `a.presence` instead of `a.blank? ? nil : a`.
a.present? ? a : b
^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `a.presence || b` instead of `a.present? ? a : b`.
field.destroy if field.present?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `field.presence&.destroy` instead of `field.destroy if field.present?`.
topic.update_pinned(false) if topic.present?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `topic.presence&.update_pinned(false)` instead of `topic.update_pinned(false) if topic.present?`.
reply_to_post.present? ? reply_to_post.post_number : nil
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `reply_to_post.presence&.post_number` instead of `reply_to_post.present? ? reply_to_post.post_number : nil`.
