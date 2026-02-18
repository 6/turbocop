user.errors[:name] << 'msg'
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

user.errors[:name].clear
^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

user.errors.messages[:name] << 'msg'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

user.errors.keys
^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

user.errors.values
^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

user.errors.to_h
^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.
