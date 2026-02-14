class Foo
  def bar
    1
  end
  def baz
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    2
  end
  def qux
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    3
  end

  def quux
    4
  end
  def corge
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    5
  end
end
