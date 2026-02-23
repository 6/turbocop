STDOUT.puts('hello')
^^^^^^ Style/GlobalStdStream: Use `$stdout` instead of `STDOUT`.

hash = { out: STDERR, key: value }
              ^^^^^^ Style/GlobalStdStream: Use `$stderr` instead of `STDERR`.

def m(out = STDIN)
            ^^^^^ Style/GlobalStdStream: Use `$stdin` instead of `STDIN`.
  out.gets
end
