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
