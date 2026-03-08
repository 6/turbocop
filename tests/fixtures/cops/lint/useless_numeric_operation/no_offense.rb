x + 1
x - 1
x * 2
x / 2
x ** 2
x += 1
x *= 2
# Local variables, instance vars, constants are not bare method calls
attachment_count = 5
result = attachment_count * 1
result = @count + 0
# Float operands are not matched by RuboCop (intentional float coercion)
val *= 1.0
val += 0.0
val /= 1.0
val **= 1.0
val -= 0.0
# Instance variable op-assigns are not matched by RuboCop
@index += 0
@count -= 0
@total *= 1
@value /= 1
# Class variable op-assigns are not matched by RuboCop
@@counter += 0
@@total *= 1
# Global variable op-assigns are not matched by RuboCop
$counter += 0
$total *= 1
# Method calls with arguments are not bare method calls (RuboCop's NodePattern requires no args)
Complex(1, 2) ** 1
Integer(val) + 0
Float(val) * 1
some_method(arg) + 0
some_method(arg) * 1
