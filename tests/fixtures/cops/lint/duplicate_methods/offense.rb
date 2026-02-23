class Foo
  def bar
    1
  end

  def bar
  ^^^ Lint/DuplicateMethods: Duplicated method definition.
    2
  end
end

class Baz
  def qux
    :a
  end

  def quux
    :b
  end

  def qux
  ^^^ Lint/DuplicateMethods: Duplicated method definition.
    :c
  end
end

module MyMod
  def helper
    true
  end

  def helper
  ^^^ Lint/DuplicateMethods: Duplicated method definition.
    false
  end
end
