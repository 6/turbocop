x = [
  1,
  2,
  3
]

y = [1, 2, 3]

z = []

# special_inside_parentheses: array arg with [ on same line as (
foo([
      :bar,
      :baz
    ])

method_call(arg1, [
              :first,
              :second
            ])

expect(cli.run([
                 '--autocorrect-all',
                 '--only', 'Style/HashSyntax'
               ])).to eq(0)

create(:record, value: [
         { source_id: '1', inbox: inbox },
         { source_id: '2', inbox: inbox2 }
       ])

deeply.nested.call([
                     :a,
                     :b
                   ])

# Array with method chain uses line-relative indent
expect(x).to eq([
  'hello',
  'world'
].join("\n"))

# Array in grouping paren with operator uses line-relative indent
X = (%i[
  a
  b
] + other).freeze

# Array as RHS of % operator inside method call
gc.draw('text %d,%d %s' % [
  left.round + 2,
  header_height + 14,
  shell_quote(week_f.to_s)
])

# Indented % operator array in method body
  image.draw('rectangle %d,%d %d,%d' % [
    0, 0, width, height
  ])
