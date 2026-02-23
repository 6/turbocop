FooError = Class.new(StandardError)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

MyClass = Class.new
^^^^^^^^^^^^^^^^^^^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.

BarError = Class.new(Alchemy::Admin::PreviewUrl)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EmptyClassDefinition: Prefer a two-line class definition over `Class.new` for classes with no body.
