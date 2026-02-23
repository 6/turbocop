class Foo
  private def bar
  ^^^^^^^ Style/AccessModifierDeclarations: `private` should not be inlined in method definitions.
    puts 'bar'
  end

  protected def baz
  ^^^^^^^^^ Style/AccessModifierDeclarations: `protected` should not be inlined in method definitions.
    puts 'baz'
  end

  public def qux
  ^^^^^^ Style/AccessModifierDeclarations: `public` should not be inlined in method definitions.
    puts 'qux'
  end
end
