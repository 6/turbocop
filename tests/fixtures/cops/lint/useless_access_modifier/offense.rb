class Foo
  public
  ^^^^^^ Lint/UselessAccessModifier: Useless `public` access modifier.

  def method
  end
end

class Bar
  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.
end

class Baz
  protected
  ^^^^^^^^^ Lint/UselessAccessModifier: Useless `protected` access modifier.
end

module Qux
  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.

  def self.singleton_method
  end
end
