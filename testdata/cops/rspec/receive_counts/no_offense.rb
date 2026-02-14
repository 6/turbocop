expect(foo).to receive(:bar).once

expect(foo).to receive(:bar).twice

expect(foo).to receive(:bar).exactly(3).times

expect(foo).to receive(:bar).exactly(n).times

expect(action).to have_published_event.exactly(1).times
