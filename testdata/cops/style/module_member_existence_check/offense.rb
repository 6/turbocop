x.instance_methods.include?(method)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ModuleMemberExistenceCheck: Use `method_defined?` instead.

x.public_instance_methods.include?(method)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ModuleMemberExistenceCheck: Use `public_method_defined?` instead.

x.constants.include?(name)
  ^^^^^^^^^^^^^^^^^^^^^^^^ Style/ModuleMemberExistenceCheck: Use `const_defined?` instead.
