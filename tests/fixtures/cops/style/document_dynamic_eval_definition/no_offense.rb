class_eval <<-EOT, __FILE__, __LINE__ + 1
  def capitalize(*params, &block)
    to_str.capitalize(*params, &block)
  end
EOT

module_eval "def foo; 42; end"

instance_eval do
  def bar; end
end

eval("def #{unsafe_method}!(*params); end # def capitalize!(*params); end")

eval(<<-RUBY, nil, __FILE__, __LINE__ + 1)
  # def capitalize(*params, &block)
  #   to_str.capitalize(*params, &block)
  # end

  def #{unsafe_method}(*params, &block)
    to_str.#{unsafe_method}(*params, &block)
  end
RUBY

eval(
  # def capitalize!(*params)
  #   @dirty = true
  #   super
  # end

  <<-RUBY, nil, __FILE__, __LINE__ + 1
    def #{unsafe_method}!(*params)
      @dirty = true
      super
    end
  RUBY
)
