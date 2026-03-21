terraform {
  required_providers {
    hcloud = {
      source  = "hetznercloud/hcloud"
      version = "~> 1.45"
    }
  }
}

variable "hcloud_token" {
  description = "Hetzner Cloud API token"
  sensitive   = true
}

variable "github_runner_token" {
  description = "GitHub runner registration token (from Settings > Actions > Runners > New)"
  sensitive   = true
}

variable "github_repo" {
  description = "GitHub repository (owner/name)"
  default     = "6/nitrocop"
}

variable "ssh_public_key" {
  description = "SSH public key for server access"
}

variable "server_type" {
  description = "Hetzner server type (cpx42 = 8 vCPU/16GB Germany ~€20/mo)"
  default     = "cpx42"
}

provider "hcloud" {
  token = var.hcloud_token
}

resource "hcloud_ssh_key" "runner" {
  name       = "nitrocop-runner"
  public_key = var.ssh_public_key
}

resource "hcloud_server" "runner" {
  name        = "nitrocop-runner"
  server_type = var.server_type
  location    = "fsn1"
  image       = "ubuntu-24.04"
  ssh_keys    = [hcloud_ssh_key.runner.id]

  user_data = templatefile("${path.module}/cloud-init.yml", {
    github_repo         = var.github_repo
    github_runner_token = var.github_runner_token
  })

  public_net {
    ipv4_enabled = true
    ipv6_enabled = true
  }
}

output "server_ip" {
  value = hcloud_server.runner.ipv4_address
}

output "server_status" {
  value = hcloud_server.runner.status
}
