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

user.errors.to_xml
^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

user.errors[:name] = []
^^^^^^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

user.errors.messages[:name] = []
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

user.errors.details[:name] << {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

user.errors.details[:name].clear
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

user.errors[:name].push('msg')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

@record.errors[:name] << 'msg'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.

user.errors[:name].concat(['msg'])
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DeprecatedActiveModelErrorsMethods: Avoid manipulating ActiveModel errors as hash directly.
