alias bar foo

alias new_name old_name

alias greet hello

alias to_s inspect

# alias_method inside a block is OK (dynamic scope, can't use alias keyword)
Struct.new(:name) do
  alias_method :first_name, :name
end

# alias_method inside a Class.new block is OK
Class.new(Base) do
  alias_method :on_send, :on_int
end

# alias_method with interpolated symbols (not plain sym) is OK
TYPES.each { |type| alias_method :"on_#{type}", :on_asgn }

# Global variable aliases should not trigger (alias_method doesn't work for gvars)
alias $new_global $old_global

alias $stdout $stderr

# alias_method inside class_eval is OK (dynamic scope, alias keyword won't work)
SomeClass.class_eval do
  alias_method :new_name, :old_name
end

# alias_method inside module_eval is OK (dynamic scope, alias keyword won't work)
SomeModule.module_eval do
  alias_method :new_name, :old_name
end

# alias_method inside class_eval with self.included pattern
module SomeModule
  def self.included(base)
    base.class_eval do
      alias_method :new_method, :old_method
    end
  end
end

# alias_method with no arguments
alias_method

# alias_method with one argument
alias_method :foo

# alias_method with non-literal constant argument
alias_method :bar, FOO

# alias_method with non-literal method call argument
alias_method :baz, foo.bar

# alias_method with explicit receiver
receiver.alias_method :ala, :bala

# alias_method in self.method def
def self.setup
  alias_method :ala, :bala
end

# alias_method inside class << self inside a def (dynamic scope from def)
def configure
  class << self
    alias_method :parse_orig, :parse
    alias_method :parse, :parse_with_timeout
  end
end

# alias_method inside class << self inside a block (dynamic scope from block)
SomeClass.class_eval do
  class << self
    alias_method :connection_orig, :connection
    alias_method :connection, :connection_patched
  end
end

# alias_method inside class << obj inside a def (dynamic scope from def)
def test_something
  class << @connection
    alias_method :old_method, :table_method
    alias_method :table_method, :test_method
  end
end

# alias inside class_eval block that is inside a def should not be flagged
# (alias_method_possible? returns false because of def ancestor)
def test_transactions
  Topic.connection.class_eval do
    alias :real_commit :commit_transaction
  end
end
