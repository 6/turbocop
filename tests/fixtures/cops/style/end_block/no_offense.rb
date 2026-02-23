at_exit { puts 'Goodbye!' }

at_exit { cleanup }

at_exit do
  save_state
end

x = 1
y = 2
