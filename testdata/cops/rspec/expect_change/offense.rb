expect { run }.to change { User.count }.by(1)
                  ^^^^^^^^^^^^^^^^^^^^^ RSpec/ExpectChange: Prefer `change(User, :count)`.
expect { run }.to change { Post.count }
                  ^^^^^^^^^^^^^^^^^^^^^ RSpec/ExpectChange: Prefer `change(Post, :count)`.
expect(run).to change { Order.total }
               ^^^^^^^^^^^^^^^^^^^^^^ RSpec/ExpectChange: Prefer `change(Order, :total)`.
