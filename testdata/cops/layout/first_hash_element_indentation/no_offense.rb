x = {
  a: 1,
  b: 2
}

y = { a: 1, b: 2 }

z = {}

# Hash inside parenthesized method call (special_inside_parentheses)
# paren at col 4, expected = 4 + 1 + 2 = 7
func({
       a: 1
     })

func(x, {
       a: 1
     })

# Hash as value of keyword arg inside parenthesized call
# paren at col 10, expected = 10 + 1 + 2 = 13
Config.new('Key' => {
             val: 1
           })

# Nested hash in keyword argument value
# paren at col 4, expected = 4 + 1 + 2 = 7
mail({
       to: to_email,
       from: from_email
     })

# Index assignment does not trigger parenthesized context
# line_indent = 0, expected = 0 + 2 = 2
config['AllCops'] = {
  val: 1
}

# Hash inside array inside parenthesized call
# paren at col 4, expected = 4 + 1 + 2 = 7
func([{
       a: 1
     }])

# Brace on different line from paren uses line indent
# line_indent = 2, expected = 2 + 2 = 4
func(
  {
    a: 1
  }
)
