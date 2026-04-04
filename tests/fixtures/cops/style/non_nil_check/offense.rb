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

# Predicate method with rescue clause — inner != nil is NOT exempt
def key?(key, options = {})
  load_entry(key, options) != nil
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NonNilCheck: Prefer `!load_entry(key, options).nil?` over `load_entry(key, options) != nil`.
rescue
  super(key, options)
end

# Predicate method with explicit begin/rescue — inner != nil is NOT exempt
def response_is_error?
  begin
    @xml.xpath("//Fault")[0] != nil
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/NonNilCheck: Prefer `!@xml.xpath("//Fault")[0].nil?` over `@xml.xpath("//Fault")[0] != nil`.
  rescue SyntaxError
    true
  end
end
