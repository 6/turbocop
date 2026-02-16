class C
  class << self
    private

    def method
      puts 'hi'
    end
  end
end

class D
  def self.method
    puts 'hi'
  end

  private_class_method :method
end
