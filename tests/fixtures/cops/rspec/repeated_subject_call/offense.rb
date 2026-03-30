RSpec.describe Foo do
  it do
    subject
    expect { subject }.to not_change { Foo.count }
             ^^^^^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
  end
end

RSpec.describe Bar do
  it do
    expect { subject }.to change { Bar.count }
    expect { subject }.to not_change { Bar.count }
             ^^^^^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
  end
end

RSpec.describe Baz do
  it do
    subject
    nested_block do
      expect { on_shard(:europe) { subject } }.to not_change { Baz.count }
                                   ^^^^^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
    end
  end
end

# Named subject alias
RSpec.describe Qux do
  subject(:bar) { do_something }

  it do
    bar
    expect { bar }.to not_change { Qux.count }
             ^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
  end
end

# Named subject used as constant path parent (mod::Params)
RSpec.describe TypeModule do
  subject(:mod) { Dry::Types.module }

  it "adds strict types as default" do
    expect(mod::Integer).to be(Dry::Types["integer"])
    expect(mod::Nominal::Integer).to be(Dry::Types["nominal.integer"])
    expect { mod::Params }.to raise_error(NameError)
             ^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
  end
end

# Named subject defined inside a pending wrapper should still be visible to nested examples
RSpec.describe PendingScope do
  context "outer" do
    pending "wrapper" do
      subject(:update_email) { do_it }

      context "inner" do
        it do
          update_email
          expect { update_email }.to change { foo.bar }
                   ^^^^^^^^^^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
        end
      end
    end
  end
end

# Shared examples should be traversed, and inline rescue should not hide the subject call
RSpec.describe SharedExamples do
  shared_examples_for "cannot create invite" do
    it do
      expect { subject }.to raise_error(StandardError)
      expect { subject rescue nil }.to change { Invite.count }
               ^^^^^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
    end
  end
end

# Parenthesized bare subject should still count as a repeated call
RSpec.describe ParenthesizedSubject do
  it do
    subject
    expect { (subject).to redirect_to dossiers_path }
              ^^^^^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
  end
end

# Multiline `expect do ... end` offenses should be anchored on the `expect` line
RSpec.describe MultilineExpect do
  subject(:post_create) { do_it }

  it do
    expect do
      post_create
      first.reload
    end.to change { first.count }

    expect do
    ^^^^^^^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
      post_create
      second.reload
    end.to change { second.count }
  end
end

# Named subject used as a keyword-hash value should not be treated like a direct call argument
RSpec.describe NamedSubjectInKeywordHash do
  subject(:token) { build_token }

  it do
    expect { create(token: token) }.to change { Item.count }
    expect { create(token: token) }.to not_change { Item.count }
                           ^^^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
  end
end
