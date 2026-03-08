Bad_name = 1
^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

Mixed_Case_Name = 2
^^^^^^^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

Bad_constant = 3
^^^^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# CamelCase-ish names assigned to non-class values should be flagged
Utagx = 0x80
^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

Hex = '0123456789abcdef'
^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

Spc = ' '[0]
^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Array literals are NOT allowed by RuboCop
Helpcmd = %w(-help --help -h)
^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

Symbols = %i(a b c)
^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

Items = [1, 2, 3]
^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Regex literals are NOT allowed by RuboCop
Pattern = /\d+/
^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

BracketDirectives = /\[\s*(?:ditto|tight)\s*\]/
^^^^^^^^^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Constant or-assignment (||=)
Foo ||= "bar"
^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

Mod::Setting ||= 42
     ^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# .freeze on interpolated string (literal receiver, flagged)
MyStr = "hello #{world}".freeze
^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Multi-assignment with constant targets
TopCase, Test2 = 5, 6
^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.
         ^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.
