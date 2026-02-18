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
notification_subscription.destroy! if notification_subscription.present?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `notification_subscription.presence&.destroy!` instead of `notification_subscription.destroy! if notification_subscription.present?`.
topic.update_pinned(false) if topic.present?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `topic.presence&.update_pinned(false)` instead of `topic.update_pinned(false) if topic.present?`.
reply_to_post.present? ? reply_to_post.post_number : nil
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `reply_to_post.presence&.post_number` instead of `reply_to_post.present? ? reply_to_post.post_number : nil`.
email.present? ? email.downcase : nil
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `email.presence&.downcase` instead of `email.present? ? email.downcase : nil`.
object.next.id if object.next.present?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `object.next.presence&.id` instead of `object.next.id if object.next.present?`.
!a.blank? ? a.foo : nil
^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `a.presence&.foo` instead of `!a.blank? ? a.foo : nil`.
a.blank? ? nil : a.bar(1, 2)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Presence: Use `a.presence&.bar(1, 2)` instead of `a.blank? ? nil : a.bar(1, 2)`.
