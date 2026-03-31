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

private m
^ Style/AccessModifierDeclarations: `private` should not be inlined in method definitions.

public  m
^ Style/AccessModifierDeclarations: `public` should not be inlined in method definitions.

class BlockedModifier
  [:a].each do |m|
    private m
    ^^^^^^^ Style/AccessModifierDeclarations: `private` should not be inlined in method definitions.
    public  m
    ^^^^^^ Style/AccessModifierDeclarations: `public` should not be inlined in method definitions.
  end
end

module Pakyow
  class Application
    class_methods do
      private def load_aspect(aspect)
      ^^^^^^^ Style/AccessModifierDeclarations: `private` should not be inlined in method definitions.
        aspect.to_s
      end

      protected def another_method
      ^^^^^^^^^ Style/AccessModifierDeclarations: `protected` should not be inlined in method definitions.
        true
      end
    end
  end
end
