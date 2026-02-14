expect { run }.to change(User, :count).by(1)
expect { run }.to change(Post, :count)
expect { run }.to change { User.sum(:points) }
expect { run }.to change { user.reload.name }
expect(run).to change(Order, :total)
Record.change { User.count }
