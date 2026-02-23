expect { foo }.to output('').to_stderr
                  ^^^^^^^^^^ RSpec/EmptyOutput: Use `not_to` instead of matching on an empty output.

expect { foo }.to output('').to_stdout
                  ^^^^^^^^^^ RSpec/EmptyOutput: Use `not_to` instead of matching on an empty output.

expect { bar }.not_to output('').to_stderr
                      ^^^^^^^^^^ RSpec/EmptyOutput: Use `not_to` instead of matching on an empty output.
