class Foo
  def self::bar
          ^^ Style/ColonMethodDefinition: Do not use `::` for defining class methods.
  end
end

class Baz
  def self::qux
          ^^ Style/ColonMethodDefinition: Do not use `::` for defining class methods.
  end
end

class Test
  def self::method_name
          ^^ Style/ColonMethodDefinition: Do not use `::` for defining class methods.
  end
end
