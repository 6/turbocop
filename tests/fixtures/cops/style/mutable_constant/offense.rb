CONST = []
        ^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST2 = {}
         ^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST3 = "hello"
         ^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

# ||= assignment is also flagged
CONST4 ||= [1, 2, 3]
           ^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST5 ||= { a: 1, b: 2 }
           ^^^^^^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST6 ||= 'str'
           ^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

# %w and %i array literals
CONST7 = %w[a b c]
         ^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST8 = %i[a b c]
         ^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST9 = %w(foo bar)
         ^^^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

# Heredoc is mutable
CONST10 = <<~HERE
          ^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.
  some text
HERE

CONST11 = <<~RUBY
          ^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.
  code here
RUBY

# Module::CONST ||= value
Mod::CONST12 ||= [1]
                 ^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.
