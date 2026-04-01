x = [
      1,
      ^^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the start of the line where the left square bracket is.
  2,
  3
]
y = [
    4,
    ^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the start of the line where the left square bracket is.
  5
]
z = [
        6,
        ^^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the start of the line where the left square bracket is.
  7
]
# Closing bracket on own line with wrong indentation inside method call parens
foo([
      :bar,
      :baz
  ])
  ^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the first position after the preceding left parenthesis.
# FN fix: Splat *[ should still use paren-relative
List.new(:BULLET, *[
  ListItem.new(nil, Paragraph.new('l1')),
  ^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the first position after the preceding left parenthesis.
  ListItem.new(nil, Paragraph.new('l2'))
])
^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the first position after the preceding left parenthesis.
# FN fix: Single-pair hash should use line-relative, not hash-key-relative
requires_login except: [
                 :index,
                 ^^^^^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the start of the line where the left square bracket is.
                 :show
               ]
               ^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the start of the line where the left bracket is.
# FN fix: String containing / should use paren-relative
Page.of_raw_data(site, '/', [
  { name: "products" },
  ^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the first position after the preceding left parenthesis.
  { name: "categories" }
])
^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the first position after the preceding left parenthesis.
# FN fix: Single-pair hash value in paren-relative — element + closing bracket at wrong indent
FactoryBot.create(:limited_admin, :groups => [
  FactoryBot.create(:google_admin_group),
  ^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the first position after the preceding left parenthesis.
])
^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the first position after the preceding left parenthesis.
# FN fix: Single-pair hash value in assert_equal — closing bracket at wrong indent
assert_equal({ "c" => [
  { "v" => 1421218800000, "f" => "Wed, Jan 14, 2015" },
  ^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the first position after the preceding left parenthesis.
  { "v" => 2, "f" => "2" },
] }, data["hits_over_time"]["rows"][1])
^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the first position after the preceding left parenthesis.
# FN fix: Empty array with wrong closing bracket indent
a << [
  ]
  ^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the start of the line where the left bracket is.

# FN fix: `%i[` keyword arg stays paren-relative even with earlier method-call args
specs = load_yaml_file(File.join(dir, basename), permitted_classes: %i[
  ELTestIvarClass
  ^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the first position after the preceding left parenthesis.
  SubELTestIvarClass
  ELTestMod::ELTestIvarClass
])
^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the first position after the preceding left parenthesis.

# FN fix: single-pair hash value stays paren-relative even with earlier method-call args
admin_in_group = FactoryBot.create(:limited_admin, :last_sign_in_at => Time.now.utc, :current_sign_in_at => Time.now.utc, :groups => [
  group,
  ^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the first position after the preceding left parenthesis.
])
^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the first position after the preceding left parenthesis.

# FN fix: single-element recipe array stays paren-relative with nested call arg
LatexToPdf.generate_pdf(IO.read(File.expand_path('../test_broken_doc_on_page_2.tex',__FILE__)),{:recipe => [
  { :command => 'xelatex', :runs => 2 }
  ^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the first position after the preceding left parenthesis.
]})
^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the first position after the preceding left parenthesis.

# FN fix: multi-element recipe array stays paren-relative with nested call arg
LatexToPdf.generate_pdf(IO.read(File.expand_path('../test_doc.tex',__FILE__)),{:recipe => [
  { :command => 'pdflatex', :extra_arguments => ['-draftmode'] },
  ^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the first position after the preceding left parenthesis.
  { :command => 'bibtex', :arguments => [] },
  { :command => 'pdflatex', :runs => 2 }
]})
^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the first position after the preceding left parenthesis.

# FN fix: explicit `.(` call parens still use paren-relative indentation
inner = "\x04\b" + array.([
  m.("0.0.0"), # @rubygems_version
  ^^ Layout/FirstArrayElementIndentation: Use 2 spaces for indentation in an array, relative to the first position after the preceding left parenthesis.
  m.(4), # @specification_version
    ])
    ^ Layout/FirstArrayElementIndentation: Indent the right bracket the same as the first position after the preceding left parenthesis.
