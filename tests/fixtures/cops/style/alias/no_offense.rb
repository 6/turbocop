alias bar foo

alias new_name old_name

alias greet hello

alias to_s inspect

alias :[] :fetch

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

# alias inside class_eval is OK (class_eval opens class scope, alias keyword valid)
SomeClass.class_eval do
  alias new_name old_name
end

# alias inside module_eval is OK (module_eval opens module scope, alias keyword valid)
SomeModule.module_eval do
  alias new_name old_name
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
