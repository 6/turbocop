class Foo
  private

  def bar
    puts 'bar'
  end

  protected

  def baz
    puts 'baz'
  end

  private :some_method
end

# Single-statement access modifiers inside blocks should be ignored
module MyModule
  singleton_methods.each { |method| private(method) }
end

# Modifier-form conditionals nested inside a one-line block should stay ignored
class Recursor
  instance_methods(true).each { |m| private m unless /^(__|object_id$)/ =~ m.to_s }
end

# Multi-statement proc bodies should stay ignored
body = proc do
  public def pub
    @a << :pub
  end

  protected def pro
    @a << :pro
  end

  private def pri
    @a << :pri
  end

  attr_reader :a
end

module Builder
  def hide(name)
    private name
  end
end

class SomeService
  included do
    private def helper
      'help'
    end
  end
end

# Inside a regular block (not class/module body)
concern do
  private def perform
    run
  end
end

# Conditional access modifiers should be skipped
# (RuboCop skips when parent is an if/unless node)
class ConditionalModifier
  if some_condition
    private :foo
  end

  unless other_condition
    protected :bar
  end
end

# Inline modifier inside conditional (parent is if_type)
class ConditionalInline
  if some_flag
    private def secret_method
      'secret'
    end
  end
end

# Multi-statement class-level begin/rescue wrappers should not preserve macro scope
class WrappedByRescue
  begin
    def before_helper
      work
    end

    private def helper line
      line
    end

    def after_helper
      work
    end
  rescue StandardError
    nil
  end
end
