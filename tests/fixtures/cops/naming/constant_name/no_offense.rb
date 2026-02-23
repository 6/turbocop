MAX_SIZE = 100
VERSION = "1.0"
MyClass = Class.new
Foo = Struct.new(:bar)
TIMEOUT_IN_SECONDS = 30

# Constant-to-constant assignment (aliasing)
Server = BaseServer
Stream = Sinatra::Helpers::Stream

# Method call with non-literal receiver
Uchar1max = (1 << 7) - 1

# Receiverless method call
Config = setup_config

# Lambda
Handler = -> { process }
MyProc = proc { do_something }
