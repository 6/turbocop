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

# Compact-style class inside multi-statement module body
module Outer
  CONSTANT = 1
  class Inner::Name
        ^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
  end
end

# Compact-style module inside multi-statement module body
module Container
  require 'something'
  module Nested::Path
         ^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
  end
end

# Multiple compact-style inside same module body
module Multi
  CONST = true
  class Alpha::Beta
        ^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
  end
  module Gamma::Delta
         ^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
  end
end

# Compact-style with cbase prefix (::) — still flagged if multi-segment
class ::Rack::MiniProfiler::SnapshotsTransporter
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

module ::FFI::Library
       ^^^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

class ::FFI::Pointer
      ^^^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

module ::FFI::WIN32
       ^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

class ::PuppetSpec::DataTypes::MyTest
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

class ::ActiveRecord::Base
      ^^^^^^^^^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

class ::ActionView::Base
      ^^^^^^^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
end

# Compact-style inside a block within a single-statement module body
module PuppetSpec
  describe "something" do
    before(:each) do
      class ::PuppetSpec::DataTypes::MyTest
            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
      end
    end
  end
end

# Compact-style class inside an if within a single-statement module body
module Underscore
  module Rails
    if defined?(::Rails) and Gem::Requirement.new('>= 3.1').satisfied_by?(Gem::Version.new(::Rails.version))
      class Rails::Engine < ::Rails::Engine
            ^^^^^^^^^^^^^ Style/ClassAndModuleChildren: Use nested module/class definitions instead of compact style.
        # this class enables the asset pipeline
      end
    end
  end
end
