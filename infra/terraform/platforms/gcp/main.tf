
terraform {
  required_version = ">= 1.6.0"

  required_providers {
    google = {
      source = "hashicorp/google"
    }
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

module "service_contract" {
  source          = "../../modules/service_contract"
  name            = var.name
  environment     = var.environment
  image           = var.image
  api_port        = var.api_port
  database_engine = var.database_engine
  redis_enabled   = var.redis_enabled
}
