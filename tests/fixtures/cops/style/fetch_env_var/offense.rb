ENV['X']
^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
x = ENV['X']
    ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
some_method(ENV['X'])
            ^^^^^^^^ Style/FetchEnvVar: Use `ENV.fetch('X', nil)` instead of `ENV['X']`.
