class User < ApplicationRecord
  def full_name
    self[:first_name]
  end

  def set_name(val)
    self[:first_name] = val
  end
end
