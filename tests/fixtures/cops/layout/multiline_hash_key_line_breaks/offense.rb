x = {
  a: 1, b: 2,
        ^ Layout/MultilineHashKeyLineBreaks: Each item in a multi-line hash must start on a separate line.
  c: 3
}

y = {
  foo: :bar, baz: :qux,
             ^^^ Layout/MultilineHashKeyLineBreaks: Each item in a multi-line hash must start on a separate line.
  quux: :corge
}

z = {
  one: 1, two: 2,
          ^^^ Layout/MultilineHashKeyLineBreaks: Each item in a multi-line hash must start on a separate line.
  three: 3
}

# Offending multiline element: :settings shares line with :app, but :defaults
# on the END line of :settings should NOT be flagged (last_seen_line algorithm).
w = {:app => {}, :settings => {:logger => ["/tmp/2.log"],
                 ^^^^^^^^^ Layout/MultilineHashKeyLineBreaks: Each item in a multi-line hash must start on a separate line.
  :logger_level => 2}, :defaults => {}}

# Multiple elements share a line, one has a multiline value; the element after
# the multiline value's end line is NOT an offense.
v = {'id' => records.first.id, 'label' => 'updated', 'action' =>
                               ^^^^^^^ Layout/MultilineHashKeyLineBreaks: Each item in a multi-line hash must start on a separate line.
                                                     ^^^^^^^^^ Layout/MultilineHashKeyLineBreaks: Each item in a multi-line hash must start on a separate line.
  {'type' => 'Field', 'attrs' => {}}, 'responders' => []}
