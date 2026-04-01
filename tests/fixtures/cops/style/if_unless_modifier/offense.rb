if x
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.
  do_something
end

unless x
^^^^^^ Style/IfUnlessModifier: Favor modifier `unless` usage when having a single-line body.
  do_something
end

if condition
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.
  foo
end

unless finished?
^^^^^^ Style/IfUnlessModifier: Favor modifier `unless` usage when having a single-line body.
  retry
end

# Parenthesized condition (non-assignment) should still be flagged
if (x > 0)
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.
  do_something
end

# Blank line between condition and body should still be flagged
if condition
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.

  do_something
end

# Short comment on condition line should still be flagged
if condition # short comment
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.
  do_something
end

# One-line form should be flagged
if foo; bar; end
^^ Style/IfUnlessModifier: Favor modifier `if` usage when having a single-line body.

raise 'ERROR: BDBA Scan Failed - Check BDBA Logs for More Info...' if scan_progress_resp[:products].any? { |p| p[:status] == 'F' }
^ Style/IfUnlessModifier: Modifier form of `if` makes the line too long.

raise "ERROR: Failed to import OpenAPI/Swagger spec #{openapi_spec} into Burp Suite Pro's Sitemap." if json_sitemap.nil? || json_sitemap.empty?
^ Style/IfUnlessModifier: Modifier form of `if` makes the line too long.

raise 'ERROR: Flags --include-response-codes and --exclude-response-codes cannot be used together.' if include_http_response_codes && exclude_http_response_codes
^ Style/IfUnlessModifier: Modifier form of `if` makes the line too long.

additional_http_headers = JSON.parse(additional_http_headers, symbolize_names: true) if additional_http_headers.is_a?(String)
^ Style/IfUnlessModifier: Modifier form of `if` makes the line too long.

raise 'ERROR: Jira Server Hash not found in PWN::Env.  Run i`pwn -Y default.yaml`, then `PWN::Env` for usage.' if engine.nil?
^ Style/IfUnlessModifier: Modifier form of `if` makes the line too long.

raise 'ERROR: Jira Server Hash not found in PWN::Env.  Run i`pwn -Y default.yaml`, then `PWN::Env` for usage.' if blockchain.nil?
^ Style/IfUnlessModifier: Modifier form of `if` makes the line too long.
