response.parsed_body
JSON.parse(data)
JSON.parse(file.read)
response.body
JSON.parse(request.body)
Nokogiri::XML(response.body)
# Extra arguments — RuboCop only matches exactly 1 argument
JSON.parse(response.body, symbolize_names: true)
JSON.parse(response.body, symbolize_names: false)
Nokogiri::HTML.parse(response.body, nil, "UTF-8")
Nokogiri::HTML5.parse(response.body, max_errors: 10)
