output "state_bucket_name" {
  description = "Pass this value to terraform init as the GCS backend bucket."
  value       = google_storage_bucket.terraform_state.name
}

output "terraform_state_member" {
  description = "Bucket-scoped member that remains after project-level Storage Admin is removed."
  value       = google_storage_bucket_iam_member.terraform_state_objects.member
}
