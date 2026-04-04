if condition
^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  x = 1
else
  x = 2
end

if foo
^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  bar = something
else
  bar = other_thing
end

if test
^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  result = :yes
else
  result = :no
end

# case/when with local variable assignment
case pwn_provider
^^^^^^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
when 'aws'
  config_path = 'aws.yaml'
when 'virtualbox'
  config_path = 'vbox.yaml'
else
  config_path = ''
end

# if/else with setter method assignment
if vagrant_gui == 'true'
^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  vm.gui = true
else
  vm.gui = false
end

# if/else with index setter assignment
if name.match?('.xlsx')
^^^^^^^^^^^^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  mail.attachments[name] = { content: body, transfer_encoding: :base64 }
else
  mail.attachments[name] = body
end

# case/when with setter method assignment
case level.to_s.downcase.to_sym
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
when :debug
  logger.level = Logger::DEBUG
when :error
  logger.level = Logger::ERROR
else
  logger.level = Logger::INFO
end

# case/when with instance variable assignment
case cmd_resp
^^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
when '21'
  @msg = :invalid_command
when '28'
  @msg = :card_speed_measurement_start
else
  @msg = :unknown
end
