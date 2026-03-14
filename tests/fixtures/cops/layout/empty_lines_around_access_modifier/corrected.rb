class Foo
  def bar
  end

  private

  def baz
  end

  protected

  def qux
  end

  public

  def quux
  end
end

# Access modifier with trailing comment, missing blank after
class Config
  def setup
  end

  private # internal helpers

  def helper
  end
end

# Access modifier at class opening with trailing comment, missing blank after
class Helper
  protected # only subclasses

  def action
  end
end

# Access modifier inside a block, missing blank line after
included do
  private

  def test
  end
end

# Access modifier inside a block, missing blank line before and after
included do
  def setup
  end

  private

  def test
  end
end

# Access modifier inside a brace block, missing blank line after
included {
  protected

  def test
  end
}
