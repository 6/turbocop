expect { foo }.to output('foo').to_stderr

expect { foo }.not_to output('foo').to_stderr

expect { foo }.not_to output.to_stderr

expect { foo }.to_not output.to_stderr

expect { foo }.to output("hello\n").to_stdout
