x.instance_methods.include?(method)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ModuleMemberExistenceCheck: Use `method_defined?` instead.

x.public_instance_methods.include?(method)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ModuleMemberExistenceCheck: Use `public_method_defined?` instead.

x.constants.include?(name)
  ^^^^^^^^^^^^^^^^^^^^^^^^ Style/ModuleMemberExistenceCheck: Use `const_defined?` instead.

x.included_modules.include?(SomeModule)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ModuleMemberExistenceCheck: Use `include?` instead.

x.class_variables.include?(:@@foo)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ModuleMemberExistenceCheck: Use `class_variable_defined?` instead.

included_modules.include?(InstanceMethods)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ModuleMemberExistenceCheck: Use `include?` instead.
