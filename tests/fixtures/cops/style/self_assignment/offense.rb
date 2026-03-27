x = x + 1
^^^^^^^^^ Style/SelfAssignment: Use self-assignment shorthand `+=`.

x = x - 1
^^^^^^^^^ Style/SelfAssignment: Use self-assignment shorthand `-=`.

x = x * 2
^^^^^^^^^ Style/SelfAssignment: Use self-assignment shorthand `*=`.

x = x ** 2
^^^^^^^^^^ Style/SelfAssignment: Use self-assignment shorthand `**=`.

parameters = parameters || []
^ Style/SelfAssignment: Use self-assignment shorthand `||=`.

filter = filter || block
^ Style/SelfAssignment: Use self-assignment shorthand `||=`.

filter = filter || block
^ Style/SelfAssignment: Use self-assignment shorthand `||=`.

response = response || retry_data[:error].http_response if retry_data[:error] && retry_data[:error].respond_to?("http_response")
^ Style/SelfAssignment: Use self-assignment shorthand `||=`.

results = results || Service::EnumerationResults.new;
^ Style/SelfAssignment: Use self-assignment shorthand `||=`.

signer = signer || Azure::Storage::Common::Core::Auth::SharedKey.new(
^ Style/SelfAssignment: Use self-assignment shorthand `||=`.

signer = signer || Azure::Storage::Common::Core::Auth::SharedAccessSignatureSigner.new(
^ Style/SelfAssignment: Use self-assignment shorthand `||=`.

scope = scope || Dogapi::Scope.new()
^ Style/SelfAssignment: Use self-assignment shorthand `||=`.
