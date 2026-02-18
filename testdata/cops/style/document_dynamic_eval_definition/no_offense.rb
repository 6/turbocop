class_eval <<-EOT, __FILE__, __LINE__ + 1
  def capitalize(*params, &block)
    to_str.capitalize(*params, &block)
  end
EOT

module_eval "def foo; 42; end"

instance_eval do
  def bar; end
end
