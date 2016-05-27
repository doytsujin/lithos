# -*- mode: ruby -*-
# vi: set ft=ruby :
Vagrant.configure(2) do |config|

  config.vm.box = "ubuntu/trusty64"

  config.vm.provision "shell", inline: <<-SHELL
    set -ex
    echo 'deb http://ubuntu.zerogw.com vagga-testing main' | tee /etc/apt/sources.list.d/vagga.list
    apt-get update
    apt-get install -y --force-yes vagga cgroup-lite
  SHELL

  config.vm.provision "shell", run: "always", inline: <<-SHELL
    set -ex
    ensure_dir() { [ -d $1 ] || ( mkdir $1 && chown vagrant $1 ); }
    ensure_dir /vagrant/.vagga
    ensure_dir /vagrant/target
    ensure_dir /home/vagrant/.cache/_vagga
    ensure_dir /home/vagrant/.cache/_cargo
    mount --bind /home/vagrant/.cache/_vagga /vagrant/.vagga
    mount --bind /home/vagrant/.cache/_cargo /vagrant/target
  SHELL

end
