class_eval <<-EOT, __FILE__, __LINE__ + 1
^^^^^^^^^^ Style/DocumentDynamicEvalDefinition: Add a comment block showing its appearance if interpolated.
  def #{unsafe_method}(*params, &block)
    to_str.#{unsafe_method}(*params, &block)
  end
EOT

module_eval <<-EOT
^^^^^^^^^^^ Style/DocumentDynamicEvalDefinition: Add a comment block showing its appearance if interpolated.
  def #{method_name}
    42
  end
EOT

instance_eval "def original_filename; 'stringio#{n}.txt'; end"
^^^^^^^^^^^^^ Style/DocumentDynamicEvalDefinition: Add a comment block showing its appearance if interpolated.

eval("#{controller.capitalize.singularize}.find_by_title('#{options[:title]}').url_for_me(options[:action].downcase)")
^ Style/DocumentDynamicEvalDefinition: Add a comment block showing its appearance if interpolated.

expect(current_path).to eq eval("#{page}_event_path(event)")
                           ^ Style/DocumentDynamicEvalDefinition: Add a comment block showing its appearance if interpolated.

eval "class #{self.class}; module PluginFormatters end; end"
^ Style/DocumentDynamicEvalDefinition: Add a comment block showing its appearance if interpolated.

eval("self.verify_provider_#{self[:provider].to_s.downcase}")
^ Style/DocumentDynamicEvalDefinition: Add a comment block showing its appearance if interpolated.

invalid_keys = KEYS.select{ |key| !eval("#{key.to_s.upcase}S").include?(self[key]) }
                                   ^ Style/DocumentDynamicEvalDefinition: Add a comment block showing its appearance if interpolated.

missing_keys = eval("PROVIDER_#{self[:provider].to_s.upcase}_KEYS").select{ |key| !self[self[:provider]].key?(key) }
               ^ Style/DocumentDynamicEvalDefinition: Add a comment block showing its appearance if interpolated.

eval(<<-RUBY, Object.new.send(:binding), __FILE__, __LINE__ + 1) # standard:disable Security/Eval
^ Style/DocumentDynamicEvalDefinition: Add a comment block showing its appearance if interpolated.
  def evaluate(context)
    @context = context
    #{compiled_expr}
  end
RUBY

eval(<<-RUBY, nil, __FILE__, __LINE__ + 1) # standard:disable Security/Eval
^ Style/DocumentDynamicEvalDefinition: Add a comment block showing its appearance if interpolated.
def #{method_name}(#{args})
  return super(#{args}) unless ::Datadog::Tracing.active_trace

  ::Datadog::Tracing.trace(#{span_name.inspect}) { super(#{args}) }
end
RUBY
