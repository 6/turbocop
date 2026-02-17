x.fetch(key, default_value) { block_value }
             ^^^^^^^^^^^^^ Lint/UselessDefaultValueArgument: Block supersedes default value argument.
Array.new(size, default_value) { block_value }
                ^^^^^^^^^^^^^ Lint/UselessDefaultValueArgument: Block supersedes default value argument.
hash.fetch(:foo, 'bar') { 'baz' }
                 ^^^^^ Lint/UselessDefaultValueArgument: Block supersedes default value argument.
