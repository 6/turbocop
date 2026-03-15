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

