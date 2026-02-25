x = 1
puts "hello"
my_debugger
pry_helper(x)
y = binding
# pry used as a receiver (not a debugger call)
pry.exec_hook :before_session
pry.config.correct_indent
pry.reset_eval_string
pry.eval(val)
pry.output
pry.current_binding
pry.select_prompt
# pry passed as an argument (not a debugger call)
exec_hook(:before_session, pry)
do_something(pry.output, pry.current_binding, pry)
result = [pry, other]
# other debugger method names used as receivers
code.debugger
code.byebug
code.pry
code.remote_byebug
code.irb
code.save_and_open_page
