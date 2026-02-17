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
