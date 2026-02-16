class C
  private

  def self.method1
  ^^^ Lint/IneffectiveAccessModifier: `private` (on line 2) does not make singleton methods private. Use `private_class_method` or `private` inside a `class << self` block instead.
    puts 'hi'
  end
end

class D
  protected

  def self.method2
  ^^^ Lint/IneffectiveAccessModifier: `protected` (on line 10) does not make singleton methods protected. Use `protected` inside a `class << self` block instead.
    puts 'hi'
  end
end

class E
  private

  def self.method3
  ^^^ Lint/IneffectiveAccessModifier: `private` (on line 18) does not make singleton methods private. Use `private_class_method` or `private` inside a `class << self` block instead.
    puts 'hi'
  end
end
