defined?('FooBar')
^^^^^^^^^^^^^^^^^^ Lint/UselessDefined: Calling `defined?` with a string argument will always return a truthy value.
defined?(:FooBar)
^^^^^^^^^^^^^^^^^ Lint/UselessDefined: Calling `defined?` with a symbol argument will always return a truthy value.
defined?(:foo_bar)
^^^^^^^^^^^^^^^^^^ Lint/UselessDefined: Calling `defined?` with a symbol argument will always return a truthy value.
