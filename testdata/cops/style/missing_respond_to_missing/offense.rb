class Test1
  def method_missing
  ^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  end
end

class Test2
  def self.method_missing
  ^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  end
end

class Test3
  def self.method_missing
  ^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  end

  def respond_to_missing?
  end
end

class Test4
  def self.respond_to_missing?
  end

  def method_missing
  ^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  end
end
