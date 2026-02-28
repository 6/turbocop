str.casecmp("other").zero?
str == "other"
str.downcase

# Safe navigation (&.) - can't use casecmp safely with nil
str&.downcase == "other"
str&.upcase == "OTHER"
params[:key]&.downcase == "value"
element&.tag_name&.downcase == "table"

# RHS is not a string literal, downcase/upcase call, or parenthesized string
l.downcase == update_type.to_s
tester.email.downcase == text
tag.name.downcase == normalized_query
object.name.downcase == @object_class
str.downcase == some_variable
str.upcase == another_method()
str.downcase == obj.some_method

# Interpolated strings - RuboCop only flags simple string literals
x.downcase == "hello #{name}"
x.upcase == "PREFIX_#{suffix}"

# Safe navigation on operand's downcase/upcase call
y.downcase == x&.downcase
"hello" == x&.upcase
