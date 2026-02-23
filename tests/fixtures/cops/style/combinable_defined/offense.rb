defined?(Foo) && defined?(Foo::Bar)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.

defined?(Foo::Bar) && defined?(Foo)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.

defined?(Foo::Bar) && defined?(Foo::Bar::Baz)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.
