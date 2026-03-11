user.errors.add(:name, 'msg')
user.errors.delete(:name)
errors[:name].present?
errors.messages[:name].present?
errors.details[:name].present?
errors.messages[:name].keys
errors.details[:name].keys

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
