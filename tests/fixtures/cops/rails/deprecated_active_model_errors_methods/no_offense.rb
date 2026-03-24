user.errors.add(:name, 'msg')
user.errors.delete(:name)
errors[:name].present?
errors.messages[:name].present?
errors.details[:name].present?
errors.messages[:name].keys
errors.details[:name].keys

# errors.messages returns a plain Hash — .keys/.values on it are valid
user.errors.messages.keys
user.errors.messages.values
user.errors.messages.size
user.errors.details.keys
user.errors.details.values

# errors called with arguments is not ActiveModel errors — do NOT flag
result.errors(locale: :de).to_h
result.errors(full: true).to_h
contract.call(attrs).errors(full: true).to_h.each_value { |v| v }

# Deprecated methods called WITH arguments should NOT be flagged
# (RuboCop's errors_deprecated? pattern only matches argument-less calls)
user.errors.to_xml(:skip_instruct => true)

# Bare `errors` (no explicit receiver) should NOT be flagged outside model files
errors.keys
errors.values
errors.to_h
errors.to_xml
errors[:name] << 'msg'
errors[:name].clear
errors[:name] = []
errors.messages[:name] << 'msg'
errors.messages[:name].clear
errors.messages[:name] = []
errors.details[:name] << {}
errors.details[:name].clear
errors.details[:name] = []
