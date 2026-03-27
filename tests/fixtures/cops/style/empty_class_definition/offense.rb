FooError = Class.new(StandardError)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

MyClass = Class.new
^^^^^^^^^^^^^^^^^^^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

BarError = Class.new(Alchemy::Admin::PreviewUrl)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

::PathA::B::C = Class.new
^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

::PathA::B::C = Class.new
^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

ActiveJob::QueueAdapters::DelayedAdapter = Class.new(Delayed::ActiveJobAdapter)
^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

::Foo = Class.new
^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

::Foo = Class.new
^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

::FooCommand = Class.new
^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

Win32::Service = Class.new
^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

Win32::Registry = Class.new
^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

before :all do
  ::PathA::B::D = Class.new
  ^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.
end
