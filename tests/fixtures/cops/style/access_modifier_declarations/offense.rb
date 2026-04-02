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

outer do
  before do
    FirstClass.class_eval do
      def a_method_that_calls_private_methods
        a_scoped_private_method
      end

      private def a_scoped_private_method
      ^^^^^^^ Style/AccessModifierDeclarations: `private` should not be inlined in method definitions.
        :instance_private_stuff
      end

      private

      def an_inline_private_method
        :more_instance_private_stuff
      end
    end
  end
end

class PaymentTransaction::Shopify < PaymentTransaction
  concerning :WebhookMethods do
    class_methods do
      def receive_webhook(request)
        verify_webhook!(request)
      end

      private def verify_webhook!(request)
      ^^^^^^^ Style/AccessModifierDeclarations: `private` should not be inlined in method definitions.
        request
      end
    end
  end
end

class Memoizer
  private *instance_methods(true).select { |m| m.to_s !~ /^__/ }
  ^^^^^^^ Style/AccessModifierDeclarations: `private` should not be inlined in method definitions.

  def initialize(object)
    @object = object
  end
end
