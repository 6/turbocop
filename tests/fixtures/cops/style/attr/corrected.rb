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

attr_reader(:search, :path, :options, :joins, :table_name, :column_name,
  :origin_class, :association, :attribute)

attr_reader(:title, :clause, :default)

attr_reader( :controls )

attr_reader(name) || attr_reader(name, default_value, 'diagram')

attr_reader("#{opt}-option")

File.expand_path(attr_accessor('docdir', "", true))

attr_reader(name, Marshalers::StringMarshaler.new(opts), opts)

attr_reader(name, Marshalers::BooleanMarshaler.new(opts), opts)
