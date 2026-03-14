class Foo
  def bar
  end
  private
  ^^^^^^^ Layout/EmptyLinesAroundAccessModifier: Keep a blank line before and after `private`.
  def baz
  end
  protected
  ^^^^^^^^^ Layout/EmptyLinesAroundAccessModifier: Keep a blank line before and after `protected`.
  def qux
  end
  public
  ^^^^^^ Layout/EmptyLinesAroundAccessModifier: Keep a blank line before and after `public`.
  def quux
  end
end

# Access modifier with trailing comment, missing blank after
class Config
  def setup
  end

  private # internal helpers
  ^^^^^^^ Layout/EmptyLinesAroundAccessModifier: Keep a blank line after `private`.
  def helper
  end
end

# Access modifier at class opening with trailing comment, missing blank after
class Helper
  protected # only subclasses
  ^^^^^^^^^ Layout/EmptyLinesAroundAccessModifier: Keep a blank line after `protected`.
  def action
  end
end

# Access modifier inside a block, missing blank line after
included do
  private
  ^^^^^^^ Layout/EmptyLinesAroundAccessModifier: Keep a blank line after `private`.
  def test
  end
end

# Access modifier inside a block, missing blank line before and after
included do
  def setup
  end
  private
  ^^^^^^^ Layout/EmptyLinesAroundAccessModifier: Keep a blank line before and after `private`.
  def test
  end
end

# Access modifier inside a brace block, missing blank line after
included {
  protected
  ^^^^^^^^^ Layout/EmptyLinesAroundAccessModifier: Keep a blank line after `protected`.
  def test
  end
}
