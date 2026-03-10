class User < ApplicationRecord
  def full_name
    self[:first_name]
  end

  def set_name(val)
    self[:first_name] = val
  end
end

class Topic < ApplicationRecord
  def slug
    read_attribute(:slug)
  end

  def title=(t)
    write_attribute(:title, t)
  end
end

# String argument in shadowing method should also be skipped
class Report < ApplicationRecord
  def conditions=(exp)
    write_attribute("conditions", exp)
  end

  def slug
    read_attribute("slug")
  end
end
