raise StandardError, 'message'
fail StandardError, 'message'
raise
fail
raise ArgumentError
raise MyError.new('msg')
Kernel.raise "error message"
::Kernel.raise "error message"
Kernel.fail "error message"
::Kernel.fail "error message"
