class Foo::Bar
      ^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

module Foo::Bar::Baz
       ^^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

class FooClass::BarClass
      ^^^^^^^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

module FooModule::BarModule
       ^^^^^^^^^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

class Foo::Bar < Super
      ^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

class Foo::Bar
      ^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
  class Baz
  end
end

module Foo::Bar
       ^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
  module Baz
  end
end
