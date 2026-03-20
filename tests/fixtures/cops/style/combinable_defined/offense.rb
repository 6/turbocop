defined?(Foo) && defined?(Foo::Bar)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.

defined?(Foo::Bar) && defined?(Foo)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.

defined?(Foo::Bar) && defined?(Foo::Bar::Baz)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.

defined?(foo) && defined?(foo.bar)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.

defined?(Rails) && defined?(Rails.backtrace_cleaner)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.

defined?(CGI) && defined?(CGI.escape)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.

defined?(ActiveSupport::Logger) && defined?(ActiveSupport::Logger.broadcast)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.

defined?(Spec) && defined?(Spec.configure)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.

defined?(::ActiveJob::Base) && defined?(::ActiveJob::Base.queue_name_prefix)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.

defined?(::Foo) && defined?(::Foo::Bar)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableDefined: Combine nested `defined?` calls.
