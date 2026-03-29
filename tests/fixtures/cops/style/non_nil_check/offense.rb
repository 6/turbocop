x != nil
^^^^^^^^ Style/NonNilCheck: Prefer `!x.nil?` over `x != nil`.

foo != nil
^^^^^^^^^^ Style/NonNilCheck: Prefer `!foo.nil?` over `foo != nil`.

bar.baz != nil
^^^^^^^^^^^^^^ Style/NonNilCheck: Prefer `!bar.baz.nil?` over `bar.baz != nil`.

def expired?(time = Time.now)
  expires_at != nil && time > expires_at
  ^^^^^^^^^^^^^^^^^ Style/NonNilCheck: Prefer `!expires_at.nil?` over `expires_at != nil`.
end

def numeric?(string)
  Float(string) != nil rescue false
  ^^^^^^^^^^^^^^^^^^^^ Style/NonNilCheck: Prefer `!Float(string).nil?` over `Float(string) != nil`.
end

class Test
  def self.for?(klass)
    (collections[klass] != nil) && ready?
     ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NonNilCheck: Prefer `!collections[klass].nil?` over `collections[klass] != nil`.
  end
end
