variable "project_id" {
  description = "GCP project that owns the NCN router infrastructure."
  type        = string
  default     = "ncn-router"
}

variable "region" {
  description = "Region in which to create the state bucket."
  type        = string
  default     = "europe-west2"
}

variable "state_bucket_name" {
  description = "Globally unique GCS bucket name used by the main Terraform stack."
  type        = string
  default     = "ncn-router-terraform-state"
}

variable "terraform_service_account_email" {
  description = "Pre-created CI identity that must retain object access after bootstrap."
  type        = string
  default     = null
  nullable    = true
}
