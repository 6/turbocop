something.exit
system("exit")
puts "done"
raise "a bad error"
# Non-Kernel/Process receiver is fine
my_app.exit(0)
runner.abort
