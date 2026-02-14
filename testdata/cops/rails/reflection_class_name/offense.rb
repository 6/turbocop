has_many :items, class_name: "Item"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReflectionClassName: Use a constant instead of a string for `class_name`.
belongs_to :author, class_name: "User"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReflectionClassName: Use a constant instead of a string for `class_name`.
has_one :profile, class_name: "UserProfile"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReflectionClassName: Use a constant instead of a string for `class_name`.
