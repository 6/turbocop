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

# Constant and-assignment (&&=)
OpAndLocal &&= 1
^^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Constant operator-assignment (+=)
OpAddLocal += 2
^^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Constant path and-assignment (Mod::Const &&=)
ConstSpecs::OpAndPath &&= 1
            ^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Constant path operator-assignment (Mod::Const +=)
ConstSpecs::OpAddPath += 2
            ^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Array literal with .freeze (literal receiver)
AsyncResponse = [-1, {}, []].freeze
^^^^^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Array literal with .to_set (literal receiver)
BoldStyle = [:bold].to_set
^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Array literal with + operator (literal receiver)
FieldSpec = [1, 2] + OTHER_SPEC
^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# %w array literal with .freeze
IconNames = %w(fab far fas).freeze
^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Hash literal with .freeze (literal receiver)
PageModes = { 'a' => 1, 'b' => 2 }.freeze
^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Hash literal with .merge (literal receiver)
StatusCodes = { ok: 200 }.merge(extra)
^^^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Rescue constant target
begin
  something
rescue => CapturedErr
          ^^^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.
end

# Lambda literal with literal receiver — range is a literal
PositiveRange = (0..100).select(&:positive?)
^^^^^^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Range literal receiver — ranges ARE literals in RuboCop
Letters = ('A'..'Z').to_a
^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Frozen range (literal receiver)
MyRange = (1..5).freeze
^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Array literal with block argument (&:sym) — NOT a block node
Fields = %w[name email].map(&:to_sym)
^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.

# Lambda with numbered parameters (_1) — Parser creates :numblock, not :block
# RuboCop's allowed_assignment? only includes :block, not :numblock
Positive = ->{ _1 > 0 }
^^^^^^^^ Naming/ConstantName: Use SCREAMING_SNAKE_CASE for constants.
