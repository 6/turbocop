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

# ternary with local variable assignment
opts[:encoding].nil? ? encoding = nil : encoding = opts[:encoding].to_s
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.

# ternary with setter method assignment
pi.config.pwn_ai_debug ? pi.config.pwn_ai_debug = false : pi.config.pwn_ai_debug = true
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.

# if/elsif/else with same variable assignment
if RUBY_ENGINE == 'ruby'
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  platform = 'ruby'
elsif RUBY_ENGINE == 'jruby'
  platform = 'java'
else
  platform = 'other'
end

# unless/else with same variable assignment
unless condition
^^^^^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  x = 1
else
  x = 2
end

# if/else where both branches assign the same variable (complex condition)
if content_type =~ /json/i && (response_body.is_a?(Hash) || response_body.is_a?(Array))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  response_body = JSON.generate(response_body)
else
  response_body = response_body.to_s
end

# if/else with shovel operator assignment
if @params.empty?
^^^^^^^^^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  message << " without parameters."
else
  message << " with parameters #{@params.inspect}."
end

# if/else with shovel operator assignment in a block
if i == 0
^^^^^^^^^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  this_sig_lines << options.indented(indent_level, definition)
else
  this_sig_lines << options.indented(indent_level, "#{' ' * (definition.length - 2)}| ")
end
