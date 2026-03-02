# Public method
def foo
  puts 'bar'
end

def initialize
  @x = 1
end

# Another documented method
def bar
  42
end

# Private methods don't need docs (default RequireForNonPublicMethods: false)
private

def private_method
  42
end

protected

def protected_method
  42
end

# Inline private
private def inline_private
  42
end

# Documentation for modular method
module_function def modular_method
  42
end

# Documentation for keywords method
ruby2_keywords def keyword_method
  42
end

# private_class_method is non-public, skipped by default
private_class_method def self.secret
  42
end

# TODO: fix this
# Real documentation follows the annotation
def annotated_then_doc
  42
end

# Private with indented def (common Ruby style)
class IndentedPrivate
  private
    def indented_private_method
      42
    end

  protected
    def indented_protected_method
      42
    end
end

# Private inside class << self followed by private section
module ActionCable
    class Base
      class << self
      end
      private
        def delegate_connection_identifiers
          42
        end
    end
end

# Private in nested class with different indentation
class Container
  class Nested
    private
      def deeply_nested_private
        42
      end
  end
end
