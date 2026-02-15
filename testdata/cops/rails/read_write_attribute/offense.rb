class User < ApplicationRecord
  def full_name
    read_attribute(:first_name)
    ^^^^^^^^^^^^^^ Rails/ReadWriteAttribute: Use `self[:attr]` instead of `read_attribute`.
  end

  def set_name(val)
    write_attribute(:first_name, val)
    ^^^^^^^^^^^^^^^ Rails/ReadWriteAttribute: Use `self[:attr] = val` instead of `write_attribute`.
  end

  def compute_age
    read_attribute(:age)
    ^^^^^^^^^^^^^^ Rails/ReadWriteAttribute: Use `self[:attr]` instead of `read_attribute`.
  end
end
