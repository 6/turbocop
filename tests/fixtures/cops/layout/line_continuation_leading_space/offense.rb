x = 'this text is too' \
    ' long'
     ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

y = 'this text contains a lot of' \
    '               spaces'
     ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

z = "another example" \
    " with leading space"
     ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

error = "go: example.com/tool@v1.0.0 requires\n" \
    "	github.com/example/dependency@v0.0.0-00010101000000-000000000000: invalid version"
     ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

mixed = "foo #{bar}" \
  ' long'
   ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

logger.warn("Downcasing dependency '#{name}' because deb packages " \
             " don't work so good with uppercase names")
              ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

msg = "expected #{resource} to have " \
  " the correct value"
   ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

hint = "Use #{method_name} instead of " \
  "  calling directly"
   ^^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

message = %Q{expected "#{resource}" to have parameters:} \
  "\n\n" \
  "  " + unmatched.collect { |p, h| p }
   ^^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

raise SpoofError, "IP spoofing attack?! " \
  "HTTP_CLIENT_IP=#{req.client_ip} " \
  "HTTP_X_FORWARDED_FOR=#{req.forwarded_for}" \
  " HTTP_FORWARDED=" + req.forwarded.map { "for=#{_1}" }.join(", ")
   ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

warning = "In #{resource_name} you exposed a `has_one` relationship "\
  " using the `belongs_to` class method. We think `has_one`" \
   ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.
  " is more appropriate."
   ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

if getter != ::OpenTelemetry::Context::Propagation.text_map_getter &&
    getter != ::OpenTelemetry::Common::Propagation.rack_env_getter
  Datadog.logger.error(
    "Custom getter #{getter} is not supported. Please inform the `datadog` team at " \
    ' https://github.com/DataDog/dd-trace-rb of your use case so we can best support you. Using the default ' \
     ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.
    'OpenTelemetry::Context::Propagation.text_map_getter as a fallback getter.'
  )
end

if scope&.respond_to?(method_name)
  Deprecation.warn("Calling `#{method_name}` on scope " \
    'is deprecated and will be removed in Blacklight 8. Call #to_h first if you ' \
    ' need to use hash methods (or, preferably, use your own SearchState implementation)')
     ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.
end

loan_msg = if @round.receivership_loan.positive?
             " #{current_entity.name} has spent #{@game.format_currency(@round.receivership_loan)} "\
               'on track, tokens and/or a leased train that must be repaid out of the route '\
               ' revenue. In the event that the revenue will not cover this cost, you must UNDO '\
                ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.
               'the moves that cannot be afforded.'
           else
             ''
           end
