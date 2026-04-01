if condition
^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  x = 1
else
  x = 2
end

if foo
^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  bar = something
else
  bar = other_thing
end

if test
^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  result = :yes
else
  result = :no
end

case pwn_provider
^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
when 'aws'
  config_path = './etc/userland/aws/vagrant.yaml'
when 'virtualbox'
  config_path = './etc/userland/virtualbox/vagrant.yaml'
else
  config_path = './etc/userland/vmware/vagrant.yaml'
end

if vagrant_gui == 'true'
^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  vm.gui = true
else
  vm.gui = false
end

if name.match?('.xlsx')
^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
  mail.attachments[name] = {
    content: Base64.strict_encode64(body),
    transfer_encoding: :base64
  }
else
  mail.attachments[name] = body
end

case level.to_s.downcase.to_sym
^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
when :debug
  logger.level = Logger::DEBUG
when :error
  logger.level = Logger::ERROR
else
  logger.level = Logger::UNKNOWN
end

pi.config.pager ? pi.config.pager = false : pi.config.pager = true
^ Style/ConditionalAssignment: Use the return of the conditional for variable assignment and comparison.
