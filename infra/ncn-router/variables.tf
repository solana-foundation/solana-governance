variable "project_id" {
  description = "GCP project that owns the NCN router infrastructure."
  type        = string
  default     = "ncn-router"
}

variable "region" {
  description = "Region for the subnet, address, and Artifact Registry repository."
  type        = string
  default     = "europe-west2"
}

variable "zone" {
  description = "Zone for the NCN router VM."
  type        = string
  default     = "europe-west2-c"
}

variable "domain_name" {
  description = "Hostname covered by the Cloudflare Origin CA certificate."
  type        = string
  default     = "ncn-governance.solana.com"
}

variable "deployer_service_account_email" {
  description = "Pre-created deployment identity that receives narrowly scoped access."
  type        = string
  default     = null
  nullable    = true
}

variable "subnet_cidr" {
  description = "Private IPv4 range for the router subnet."
  type        = string
  default     = "10.20.0.0/24"

  validation {
    condition     = can(cidrnetmask(var.subnet_cidr))
    error_message = "subnet_cidr must be a valid IPv4 CIDR."
  }
}

variable "cloudflare_ipv4_ranges" {
  description = "Cloudflare IPv4 proxy ranges allowed to reach nginx on TCP 443."
  type        = set(string)
  default = [
    "103.21.244.0/22",
    "103.22.200.0/22",
    "103.31.4.0/22",
    "104.16.0.0/13",
    "104.24.0.0/14",
    "108.162.192.0/18",
    "131.0.72.0/22",
    "141.101.64.0/18",
    "162.158.0.0/15",
    "172.64.0.0/13",
    "173.245.48.0/20",
    "188.114.96.0/20",
    "190.93.240.0/20",
    "197.234.240.0/22",
    "198.41.128.0/17",
  ]
}
