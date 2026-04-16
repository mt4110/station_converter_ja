output "service_contract" {
  value = {
    name            = var.name
    environment     = var.environment
    image           = var.image
    api_port        = var.api_port
    database_engine = var.database_engine
    redis_enabled   = var.redis_enabled
  }
}
