[1, 2].each { |x| puts x }
[1, 2].map { |x| x * 2 }
foo.select { |x| x > 1 }
escape_html = ->(str) { str.gsub("&", "&amp;") }
has_many :versions, -> { order("id ASC") }, class_name: "Foo"
action = -> { puts "hello" }

p(class: 'intro') { "
hello
"}

p { "
hello
"}

p(class: 'conclusion') { "
hello
"}

p(class: 'legend') { "
hello
"}

audit_options { {
  foo: :bar
}}

let(:domains) { [
  { domain: 'example.com' }
]}

let(:source) { <<-CODE
body
CODE
}

before { FlavourSaver.register_helper(:repeat) do |a, &block|
  a.times { block.call }
end}
