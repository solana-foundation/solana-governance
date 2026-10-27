locals {
  name          = "ncn-router"
  instance_name = "ncn-router-managed"
  deployer_service_account_email = coalesce(
    var.deployer_service_account_email,
    "ncn-router-deployer@${var.project_id}.iam.gserviceaccount.com",
  )
  deployer_member = "serviceAccount:${local.deployer_service_account_email}"
  startup_script = templatefile("${path.module}/startup.sh.tftpl", {
    cloudflare_origin_pull_ca_base64 = base64encode(file("${path.module}/cloudflare-origin-pull-ca.pem"))
    deploy_script_base64             = base64encode(file("${path.module}/scripts/ncn-router-deploy"))
    domain_name                      = var.domain_name
  })

  required_services = toset([
    "artifactregistry.googleapis.com",
    "compute.googleapis.com",
    "iam.googleapis.com",
    "iap.googleapis.com",
    "logging.googleapis.com",
    "oslogin.googleapis.com",
  ])
}

resource "google_project_service" "required" {
  for_each = local.required_services

  project            = var.project_id
  service            = each.value
  disable_on_destroy = false
}

resource "google_compute_network" "router" {
  name                    = local.name
  project                 = var.project_id
  auto_create_subnetworks = false
  routing_mode            = "REGIONAL"

  depends_on = [google_project_service.required]
}

resource "google_compute_subnetwork" "router" {
  name                     = local.name
  project                  = var.project_id
  region                   = var.region
  network                  = google_compute_network.router.id
  ip_cidr_range            = var.subnet_cidr
  private_ip_google_access = true
  stack_type               = "IPV4_ONLY"
}

resource "google_compute_address" "router" {
  name         = local.name
  project      = var.project_id
  region       = var.region
  address_type = "EXTERNAL"
  network_tier = "PREMIUM"

  depends_on = [google_project_service.required]
}

resource "google_compute_firewall" "cloudflare_https" {
  name          = "ncn-router-allow-https-cloudflare"
  project       = var.project_id
  network       = google_compute_network.router.name
  direction     = "INGRESS"
  priority      = 1000
  source_ranges = sort(tolist(var.cloudflare_ipv4_ranges))
  target_tags   = [local.name]

  allow {
    protocol = "tcp"
    ports    = ["443"]
  }

  log_config {
    metadata = "INCLUDE_ALL_METADATA"
  }
}

resource "google_compute_firewall" "iap_ssh" {
  name          = "allow-ssh-iap"
  project       = var.project_id
  network       = google_compute_network.router.name
  direction     = "INGRESS"
  priority      = 1000
  source_ranges = ["35.235.240.0/20"]
  target_tags   = [local.name]

  allow {
    protocol = "tcp"
    ports    = ["22"]
  }

  log_config {
    metadata = "INCLUDE_ALL_METADATA"
  }
}

resource "google_artifact_registry_repository" "router" {
  location      = var.region
  project       = var.project_id
  repository_id = local.name
  description   = "Production NCN router images"
  format        = "DOCKER"

  depends_on = [google_project_service.required]
}

resource "google_service_account" "runtime" {
  project      = var.project_id
  account_id   = "ncn-router-runtime"
  display_name = "NCN router VM runtime"
  description  = "Reads NCN router images and writes VM logs; not used by CI."

  depends_on = [google_project_service.required]
}

resource "google_artifact_registry_repository_iam_member" "runtime_reader" {
  project    = var.project_id
  location   = google_artifact_registry_repository.router.location
  repository = google_artifact_registry_repository.router.name
  role       = "roles/artifactregistry.reader"
  member     = "serviceAccount:${google_service_account.runtime.email}"
}

resource "google_project_iam_member" "runtime_log_writer" {
  project = var.project_id
  role    = "roles/logging.logWriter"
  member  = "serviceAccount:${google_service_account.runtime.email}"
}

# Bootstrap changes must create a new VM. Google's Compute Engine will
# update startup metadata in place, but only executes the change during 
# boot. The static IP is a separate resource and remains attached to 
#the replacement instance.
resource "terraform_data" "router_bootstrap" {
  triggers_replace = [local.startup_script]
}

resource "google_compute_instance" "router" {
  name         = local.instance_name
  project      = var.project_id
  zone         = var.zone
  machine_type = "e2-small"
  tags         = [local.name]

  allow_stopping_for_update = true

  boot_disk {
    auto_delete = true

    initialize_params {
      image = "projects/debian-cloud/global/images/family/debian-12"
      size  = 20
      type  = "pd-balanced"
    }
  }

  network_interface {
    subnetwork = google_compute_subnetwork.router.id

    access_config {
      nat_ip       = google_compute_address.router.address
      network_tier = "PREMIUM"
    }
  }

  metadata = {
    block-project-ssh-keys = "TRUE"
    enable-oslogin         = "TRUE"
  }

  metadata_startup_script = local.startup_script

  lifecycle {
    replace_triggered_by = [terraform_data.router_bootstrap]
  }

  service_account {
    email  = google_service_account.runtime.email
    scopes = ["https://www.googleapis.com/auth/cloud-platform"]
  }

  shielded_instance_config {
    enable_integrity_monitoring = true
    enable_secure_boot          = true
    enable_vtpm                 = true
  }

  labels = {
    application = "ncn-router"
    environment = "production"
    managed-by  = "terraform"
  }

  depends_on = [
    google_artifact_registry_repository_iam_member.runtime_reader,
    google_project_iam_member.runtime_log_writer,
  ]
}

# The deployer can publish only to this repository.
resource "google_artifact_registry_repository_iam_member" "deployer_writer" {
  project    = var.project_id
  location   = google_artifact_registry_repository.router.location
  repository = google_artifact_registry_repository.router.name
  role       = "roles/artifactregistry.writer"
  member     = local.deployer_member
}

# Project scope supplies compute.projects.get for gcloud while OS Login and IAP
# still determine whether the identity can enter a particular instance.
resource "google_project_iam_member" "deployer_os_admin_login" {
  project = var.project_id
  role    = "roles/compute.osAdminLogin"
  member  = local.deployer_member
}

# Instance scope prevents this otherwise broad predefined role from creating VMs.
resource "google_compute_instance_iam_member" "deployer_instance_admin" {
  project       = var.project_id
  zone          = google_compute_instance.router.zone
  instance_name = google_compute_instance.router.name
  role          = "roles/compute.instanceAdmin.v1"
  member        = local.deployer_member

  # Instance IAM policies are deleted with the VM. Recreate this binding when
  # a bootstrap change replaces the otherwise identically named instance.
  lifecycle {
    replace_triggered_by = [google_compute_instance.router]
  }
}

resource "google_iap_tunnel_instance_iam_member" "deployer_iap_ssh" {
  project  = var.project_id
  zone     = google_compute_instance.router.zone
  instance = google_compute_instance.router.name
  role     = "roles/iap.tunnelResourceAccessor"
  member   = local.deployer_member

  condition {
    title       = "ssh-only"
    description = "Permit IAP TCP forwarding only to SSH."
    expression  = "destination.port == 22"
  }

  # The instance-scoped IAP policy must follow VM replacement too.
  lifecycle {
    replace_triggered_by = [google_compute_instance.router]
  }
}

resource "google_service_account_iam_member" "deployer_runtime_user" {
  service_account_id = google_service_account.runtime.name
  role               = "roles/iam.serviceAccountUser"
  member             = local.deployer_member
}
