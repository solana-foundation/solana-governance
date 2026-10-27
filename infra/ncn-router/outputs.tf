output "external_ipv4_address" {
  description = "Create or update the Cloudflare proxied A record with this address."
  value       = google_compute_address.router.address
}

output "artifact_registry_image" {
  description = "Base image path; deployments append an immutable sha256 digest."
  value       = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.router.repository_id}/ncn-router"
}

output "instance_name" {
  description = "Compute Engine instance reached through IAP."
  value       = google_compute_instance.router.name
}

output "runtime_service_account_email" {
  description = "Least-privilege identity attached to the VM."
  value       = google_service_account.runtime.email
}
