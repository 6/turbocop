if cond
^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  foo
end

if x > 1
^^^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  bar
end

case status
^^^^^^^^^^^ Style/MissingElse: `case` condition requires an `else`-clause.
when :active
  activate
end

# elsif without final else — offense on the last elsif
if x > 1
  bar
elsif x < 0
^^^^^^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  baz
end

# Multiple elsif without final else — offense on LAST elsif only
if cond_1
  one
elsif cond_2
  two
elsif cond_3
^^^^^^^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  three
end

while ppid != '1'
  if (agent = agents[ppid])
  ^^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
    ENV['SSH_AUTH_SOCK'] = agent
    break
  end
  File.open("/proc/#{ppid}/status", "r") do |file|
    ppid = file.read().match(/PPid:\s+(\d+)/)[1]
  end
end

def restart_service(service_name)
  service_running = run_context.resource_collection.lookup("service[#{service_name}]") rescue nil

  if service_running
  ^^^^^^^^^^^^^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
    execute "trigger-notify-restart-#{service_name}" do
      command "true"
      notifies :restart, "service[#{service_name}]"
      only_if "true"
    end
  end
end

if node['ariadne']['clean']
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  project = node['ariadne']['project']

  execute "chmod -R 777 /mnt/www/html/#{project}" do
    only_if "test -d /mnt/www/html/#{project}"
  end

  %W{
    /vagrant/data/profiles/#{project}
    /mnt/www/html/#{project}
  }.each do |dir|
    directory dir do
      recursive true
      action :delete
      only_if "test -d #{dir}"
    end
  end
end

if node['instance_role'] == 'vagrant'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingElse: `if` condition requires an `else`-clause.
  ::Chef::Resource::RubyBlock.send(:include, Ariadne::Helpers)
  ruby_block "Give root access to the forwarded ssh agent" do
    block do
      give_ssh_agent_root
    end
  end
end

case node['platform']
^^^^^^^^^^^^^^^^^^^^^ Style/MissingElse: `case` condition requires an `else`-clause.
when "debian", "ubuntu"
  package "php5-memcached" do
    action :install
  end
end
