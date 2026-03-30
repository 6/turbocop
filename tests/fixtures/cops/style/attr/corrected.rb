class Foo
  attr_accessor :writable
end

class Bar
  attr_reader :one, :two, :three
end

class Baz
  attr_reader :name
end

class Qux
  attr_reader :readable
end

class Parenthesized
  attr_reader(:name)
end

module FortyFacets
  class FilterDefinition
    attr_reader(
      :search, :path, :options, :joins, :table_name, :column_name,
      :origin_class, :association, :attribute
    )
  end
end

module SKUI
  module ControlManager
    attr_reader( :controls )
  end
end

module Asciidoctor
  module Diagram
    module DiagramSource
      def global_attr(name, default_value = nil)
        attr_reader(name)
        attr_reader(name, default_value, 'diagram')
      end

      def opt(opt)
        attr_reader("#{opt}-option")
      end

      def base_dir
        File.expand_path(attr_accessor('docdir'))
      end
    end
  end
end

module Aws
  module Record
    module Attributes
      module ClassMethods
        def string_attr(name, opts = {})
          attr_reader(name, Marshalers::StringMarshaler.new(opts), opts)
        end

        def boolean_attr(name, opts = {})
          attr_reader(name, Marshalers::BooleanMarshaler.new(opts), opts)
        end
      end
    end
  end
end
