class Foo
  attr :writable, true
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_accessor` instead.
end

class Bar
  attr :one, :two, :three
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
end

class Baz
  attr :name
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
end

class Qux
  attr :readable, false
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
end

class Parenthesized
  attr(:name)
  ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
end

module FortyFacets
  class FilterDefinition
    attr(
    ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
      :search, :path, :options, :joins, :table_name, :column_name,
      :origin_class, :association, :attribute
    )
  end
end

module SKUI
  module ControlManager
    attr( :controls )
    ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
  end
end

module Asciidoctor
  module Diagram
    module DiagramSource
      def global_attr(name, default_value = nil)
        attr(name)
        ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
        attr(name, default_value, 'diagram')
        ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
      end

      def opt(opt)
        attr("#{opt}-option")
        ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
      end

      def base_dir
        File.expand_path(attr('docdir', "", true))
                         ^^^^ Style/Attr: Do not use `attr`. Use `attr_accessor` instead.
      end
    end
  end
end

module Aws
  module Record
    module Attributes
      module ClassMethods
        def string_attr(name, opts = {})
          attr(name, Marshalers::StringMarshaler.new(opts), opts)
          ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
        end

        def boolean_attr(name, opts = {})
          attr(name, Marshalers::BooleanMarshaler.new(opts), opts)
          ^^^^ Style/Attr: Do not use `attr`. Use `attr_reader` instead.
        end
      end
    end
  end
end
