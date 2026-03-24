foo_1 = 1
^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
bar_2 = 2
^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
baz_12 = 3
^^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.

def some_method_1; end
    ^^^^^^^^^^^^^^ Naming/VariableNumber: Use normalcase for method name numbers.

:some_sym_1
 ^^^^^^^^^^ Naming/VariableNumber: Use normalcase for symbol numbers.

def func(arg_1); end
         ^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.

# Compound assignment operators
config_1 ||= load
^^^^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
@data_1 ||= fetch
^^^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
counter_1 += 1
^^^^^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
val_1, val_2 = arr
^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
       ^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
@@class_var_1 &&= false
^^^^^^^^^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
$global_var_1 += 10
^^^^^^^^^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
@instance_var_1 += 5
^^^^^^^^^^^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
@@class_var_2 ||= nil
^^^^^^^^^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.

# Sigiled variables with implicit-param-like names are NOT implicit params
# RuboCop's \A_\d+\z regex checks the full name including sigil
@_1 = 1
^^^ Naming/VariableNumber: Use normalcase for variable numbers.
@@_1 = 1
^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
$_1 = 1
^^^ Naming/VariableNumber: Use normalcase for variable numbers.

# Empty string hash key — with TargetRubyVersion 4.0, Parser gem creates
# :sym (not :dsym) for hash-key empty symbols, so RuboCop's on_sym fires
# and the normalcase regex fails on empty strings
{"": true}
 ^^ Naming/VariableNumber: Use normalcase for symbol numbers.

# Rescue exception variables
begin
rescue => error_2
          ^^^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
end
begin
rescue NoMethodError => pre_cucumber_0_4
                        ^^^^^^^^^^^^^^^^ Naming/VariableNumber: Use normalcase for variable numbers.
end
