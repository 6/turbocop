END { puts 'Goodbye!' }
^^^ Style/EndBlock: Avoid the use of `END` blocks. Use `Kernel#at_exit` instead.

END { cleanup }
^^^ Style/EndBlock: Avoid the use of `END` blocks. Use `Kernel#at_exit` instead.

END { save_state }
^^^ Style/EndBlock: Avoid the use of `END` blocks. Use `Kernel#at_exit` instead.
