two = 1 + 1 # A trailing inline comment
            ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.
x = 42 # meaning of life
       ^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.
foo(bar) # call foo
         ^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

=begin
^ Style/InlineComment: Avoid trailing inline comments.
=end

value = 1
=begin
^ Style/InlineComment: Avoid trailing inline comments.
=end

it { described_class.lint traits: true } # rubocop: disable RSpec/NoExpectationExample
                                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

.where(courses_users: { course_id: course_ids, role: CoursesUsers::Roles::STUDENT_ROLE }) # rubocop: disable Layout/LineLength
                                                                                          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

Redmine::Plugin.all.each do |plugin| # rubocop: disable Rails/FindEach
                                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

{ unicode: "&#x#{values[:unicode]};".html_safe, label: values[:label] } # rubocop: disable Rails/OutputSafety
                                                                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

emoji = TanukiEmoji.find_by_alpha_code emoji_code # rubocop: disable Rails/DynamicFindBy
                                                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

def entity_headline(object_name:, type:, capitalize: true, obj: nil) # rubocop: disable Lint/UnusedMethodArgument
                                                                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

emoji = TanukiEmoji.find_by_alpha_code name # rubocop: disable Rails/DynamicFindBy
                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

emoji = TanukiEmoji.find_by_codepoints moji # rubocop: disable Rails/DynamicFindBy
                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

def validate_file(key, file) #  rubocop:disable Naming/PredicateMethod
                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

def find_model(options, id:, **) #rubocop:disable Lint/DuplicateMethods
                                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

def find_model(options, id:, **) # rubocop: disable Lint/DuplicateMethods
                                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

else # 'windows'
     ^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

else # 'windows'
     ^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.

text = <<~HEREDOC
  some text #{
    if true
      "yes"
    else # inline in heredoc interpolation
         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InlineComment: Avoid trailing inline comments.
      "no"
    end
  }
HEREDOC
