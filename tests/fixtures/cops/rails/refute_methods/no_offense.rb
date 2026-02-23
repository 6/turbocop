assert_not_equal expected, actual
assert_not_nil value
assert_not result
assert_not_match(/pattern/, str)
assert_not_empty []

# refute_pattern is NOT in the known corrections list — should not be flagged
refute_pattern { record => { name: "test" } }
refute_pattern { h1 => { name: "h1", content: "Goodbye" } }
