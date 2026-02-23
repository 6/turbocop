$stderr.puts('hello')
^^^^^^^^^^^^^^^^^^^^^ Style/StderrPuts: Use `warn` instead of `$stderr.puts` to allow such output to be disabled.

STDERR.puts('hello')
^^^^^^^^^^^^^^^^^^^^ Style/StderrPuts: Use `warn` instead of `STDERR.puts` to allow such output to be disabled.

::STDERR.puts('hello')
^^^^^^^^^^^^^^^^^^^^^^ Style/StderrPuts: Use `warn` instead of `::STDERR.puts` to allow such output to be disabled.
