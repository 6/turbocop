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

attr(:search, :path, :options, :joins, :table_name, :column_name,
  :origin_class, :association, :attribute)
# nitrocop-expect: 17:0 Style/Attr: Do not use `attr`. Use `attr_reader` instead.

attr(:title, :clause, :default)
# nitrocop-expect: 20:0 Style/Attr: Do not use `attr`. Use `attr_reader` instead.

attr( :controls )
# nitrocop-expect: 22:0 Style/Attr: Do not use `attr`. Use `attr_reader` instead.

attr(name) || attr(name, default_value, 'diagram')
# nitrocop-expect: 24:0 Style/Attr: Do not use `attr`. Use `attr_reader` instead.
# nitrocop-expect: 24:14 Style/Attr: Do not use `attr`. Use `attr_reader` instead.

attr("#{opt}-option")
# nitrocop-expect: 26:0 Style/Attr: Do not use `attr`. Use `attr_reader` instead.

File.expand_path(attr('docdir', "", true))
# nitrocop-expect: 28:17 Style/Attr: Do not use `attr`. Use `attr_accessor` instead.

attr(name, Marshalers::StringMarshaler.new(opts), opts)
# nitrocop-expect: 30:0 Style/Attr: Do not use `attr`. Use `attr_reader` instead.

attr(name, Marshalers::BooleanMarshaler.new(opts), opts)
# nitrocop-expect: 32:0 Style/Attr: Do not use `attr`. Use `attr_reader` instead.
