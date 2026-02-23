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
