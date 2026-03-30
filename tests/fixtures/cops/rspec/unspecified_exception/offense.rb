RSpec.describe User do
  it 'raises an error' do
    expect { raise StandardError }.to raise_error
                                      ^^^^^^^^^^^ RSpec/UnspecifiedException: Specify the exception being captured.
  end

  it 'raises an exception' do
    expect { raise StandardError }.to raise_exception
                                      ^^^^^^^^^^^^^^^ RSpec/UnspecifiedException: Specify the exception being captured.
  end

  it 'chains' do
    expect { foo }.to raise_error.and change { bar }
                      ^^^^^^^^^^^ RSpec/UnspecifiedException: Specify the exception being captured.
  end
end

expect { GitlabCtl::Backup.perform }.to output(/Could not find '#{etc_path}' directory. Is your package installed correctly?/).to_stderr.and raise_error
                                                                                                                                             ^^^^^^^^^^^ RSpec/UnspecifiedException: Specify the exception being captured.
