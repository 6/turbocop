str.casecmp("other").zero?
str == "other"
str.downcase
str.downcase != 'string'
str.upcase != 'string'
obj.method == str.downcase

# Safe navigation (&.) - can't use casecmp safely with nil
str&.downcase == "other"
str&.upcase == "OTHER"
params[:key]&.downcase == "value"
element&.tag_name&.downcase == "table"
