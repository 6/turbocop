some_method(
  a,
  b
)

some_method(a, b, c)

foo(
  bar
)

other_method(a, b)

x = call(1, 2, 3)

# Over-indented args: `)` at first_arg_indent - 2
val = store.fetch(
    "foo",
    bar: 1
  )

def some_method(a,
                b,
                c
               )
end

# Mixed tab/space code (loomio pattern): `)` correctly indented
    			u = Record.create!(
    				name: name,
    				email: email
  				)

# Tab-indented args on same line as `(`, not aligned (webistrano pattern)
					opts.on("-l", "--logger [STDERR|STDOUT|file]",
						"Choose logger method."
					) do |value|
						puts value
					end

# Multiple args where first is empty hash: `)` at line indentation
assert_search_matches({}, {
    "nonmatching.json" => "value",
  },
  {'key' => '4'}
)

# Grouped expression with correctly indented )
w = x * (
  y + z
)

# Single-line grouped expression (no hanging paren)
result = (a + b)
