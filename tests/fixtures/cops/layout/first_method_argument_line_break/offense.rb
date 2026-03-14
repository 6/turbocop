foo(bar,
    ^^^ Layout/FirstMethodArgumentLineBreak: Add a line break before the first argument of a multi-line method call.
  baz)

something(first,
          ^^^^^ Layout/FirstMethodArgumentLineBreak: Add a line break before the first argument of a multi-line method call.
  second,
  third)

method_call(arg1,
            ^^^^ Layout/FirstMethodArgumentLineBreak: Add a line break before the first argument of a multi-line method call.
  arg2)

super(bar,
      ^^^ Layout/FirstMethodArgumentLineBreak: Add a line break before the first argument of a multi-line method call.
  baz)

# Block argument (&) on a separate line
client.send(request_method, :post,
            ^^^^^^^^^^^^^^ Layout/FirstMethodArgumentLineBreak: Add a line break before the first argument of a multi-line method call.
            &@read_body_chunk_block)

# Block argument as only second arg on next line
process(data,
        ^^^^ Layout/FirstMethodArgumentLineBreak: Add a line break before the first argument of a multi-line method call.
        &handler)

# Multiple args with block argument spanning lines
execute(cmd, env, opts,
        ^^^ Layout/FirstMethodArgumentLineBreak: Add a line break before the first argument of a multi-line method call.
        &callback)
