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
