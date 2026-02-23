class Foo
private
^^^^^^^ Layout/AccessModifierIndentation: Indent access modifiers like `private`.
  def bar; end
end

class Baz
protected
^^^^^^^^^ Layout/AccessModifierIndentation: Indent access modifiers like `protected`.
  def qux; end
end

class Quux
public
^^^^^^ Layout/AccessModifierIndentation: Indent access modifiers like `public`.
  def corge; end
end
