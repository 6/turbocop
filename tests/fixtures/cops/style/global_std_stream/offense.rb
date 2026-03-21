STDOUT.puts('hello')
^^^^^^ Style/GlobalStdStream: Use `$stdout` instead of `STDOUT`.

hash = { out: STDERR, key: value }
              ^^^^^^ Style/GlobalStdStream: Use `$stderr` instead of `STDERR`.

def m(out = STDIN)
            ^^^^^ Style/GlobalStdStream: Use `$stdin` instead of `STDIN`.
  out.gets
end

# FN #4, #5, #7, #8: assignment to non-std gvar where const appears on RHS
$stderr = STDOUT
          ^^^^^^ Style/GlobalStdStream: Use `$stdout` instead of `STDOUT`.

$stdout = STDERR
          ^^^^^^ Style/GlobalStdStream: Use `$stderr` instead of `STDERR`.

# FN #9, #10, #11: multi-assignment with std stream constant on RHS
$stderr = @stderr =  STDERR
                     ^^^^^^ Style/GlobalStdStream: Use `$stderr` instead of `STDERR`.

$stdin  = @stdin  =  STDIN
                     ^^^^^^ Style/GlobalStdStream: Use `$stdin` instead of `STDIN`.

$stdout = @stdout =  STDOUT
                     ^^^^^^^ Style/GlobalStdStream: Use `$stdout` instead of `STDOUT`.
