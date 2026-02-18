user.errors.add(:name, 'msg')
user.errors.delete(:name)
errors[:name].present?
errors.messages[:name].present?
errors.details[:name].present?
errors.messages[:name].keys
