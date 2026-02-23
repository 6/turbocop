ERB.new(str, nil, '-', '@output_buffer')
             ^^^ Lint/ErbNewArguments: Passing safe_level with the 2nd argument of `ERB.new` is deprecated. Do not use it, and specify other arguments as keyword arguments.
                  ^^^ Lint/ErbNewArguments: Passing trim_mode with the 3rd argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, trim_mode: '-')` instead.
                       ^^^^^^^^^^^^^^^^^ Lint/ErbNewArguments: Passing eoutvar with the 4th argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, eoutvar: '@output_buffer')` instead.
ERB.new(str, nil)
             ^^^ Lint/ErbNewArguments: Passing safe_level with the 2nd argument of `ERB.new` is deprecated. Do not use it, and specify other arguments as keyword arguments.
ERB.new(str, nil, '-')
             ^^^ Lint/ErbNewArguments: Passing safe_level with the 2nd argument of `ERB.new` is deprecated. Do not use it, and specify other arguments as keyword arguments.
                  ^^^ Lint/ErbNewArguments: Passing trim_mode with the 3rd argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, trim_mode: '-')` instead.
