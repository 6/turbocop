defined?(Foo)
defined?(Foo) && defined?(Bar)
defined?(Foo) && defined?(Foo)
defined?(Foo) && defined?(::Foo)
x = defined?(Foo)
y = 1
# Skipping nesting levels — not combinable
defined?(Foo) && defined?(Foo::Bar::Baz)
defined?(RSpec) && defined?(RSpec::Core::RakeTask)
defined?(Padrino) && defined?(Padrino::Routing::InstanceMethods)
defined?(Google) && defined?(Google::Apis::CalendarV3)
# Mixed non-defined? terms — not combinable
foo && defined?(Foo) && bar && defined?(Foo::Bar)
# Different cbase
defined?(::Foo) && defined?(Foo::Bar)
# Same namespace but different nesting
defined?(Foo::Bar) && defined?(Foo::Baz)
# Skipping nesting with cbase
defined?(::Padrino) && defined?(::Padrino::PathRouter::Router)
defined?(Rails) && defined?(Rails::VERSION::STRING)
# Negation
defined?(Foo) && !defined?(Foo::Bar)
# Method chains skipping levels
defined?(UIScreen) && defined?(UIScreen.mainScreen.traitCollection)
