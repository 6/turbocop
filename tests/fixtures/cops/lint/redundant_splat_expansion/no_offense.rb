c = [1, 2, 3]
a = *c
a, b = *c
a, *b = *c
a = *1..10
a = ['a']

# AllowPercentLiteralArrayArgument: true (default)
delegate(*%i[values_for_type custom?], to: :@nil)
do_something(*%w[foo bar baz])
remove_columns :table, *%i[col1 col2 col3], type: :boolean
