variable "name" {
  type = string
}

variable "environment" {
  type = string
}

variable "image" {
  type = string
}

variable "api_port" {
  type    = number
  default = 3212
}

variable "database_engine" {
  type    = string
  default = "postgres"
}

variable "redis_enabled" {
  type    = bool
  default = true
}
