create(:user)
build(:user)
build :user, username: "NAME"
create("users/internal")
create user: :foo
build user: :foo
generate("
  class Foo
    def bar; end
  end")
generate("class Foo\ndef bar; end\nend")
