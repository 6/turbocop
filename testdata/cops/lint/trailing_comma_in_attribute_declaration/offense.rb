class Foo
  attr_reader :foo,
  ^^^^^^^^^^^^^^^^^^^ Lint/TrailingCommaInAttributeDeclaration: Avoid leaving a trailing comma in attribute declarations.

  def bar
    puts "Unreachable."
  end
end

class Bar
  attr_accessor :x,
  ^^^^^^^^^^^^^^^^^^^^ Lint/TrailingCommaInAttributeDeclaration: Avoid leaving a trailing comma in attribute declarations.

  def baz
    puts "Unreachable."
  end
end

class Baz
  attr_writer :y,
  ^^^^^^^^^^^^^^^^^^ Lint/TrailingCommaInAttributeDeclaration: Avoid leaving a trailing comma in attribute declarations.

  def qux
    puts "Unreachable."
  end
end
