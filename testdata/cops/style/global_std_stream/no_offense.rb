$stdout.puts('hello')
$stderr.puts('hello')
$stdin.gets
SOME_CONST.puts('hello')
Foo::STDOUT.puts('hello')
Foo::Bar::STDERR.puts('hello')

# Global variable assignment to std stream constant is OK
$stdout = STDOUT
$stderr = STDERR
$stdin = STDIN
