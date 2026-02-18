foo.reduce { |a, b| a + b }
             ^^^^^^ Style/SingleLineBlockParams: Name `reduce` block params `|acc, elem|`.

foo.inject { |x, y| x + y }
             ^^^^^^ Style/SingleLineBlockParams: Name `inject` block params `|acc, elem|`.

bar.reduce { |sum, item| sum + item }
             ^^^^^^^^^^^ Style/SingleLineBlockParams: Name `reduce` block params `|acc, elem|`.
