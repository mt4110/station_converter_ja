variable "region" {
  type    = string
  default = "ap-northeast-1"
}

variable "name" {
  type    = string
  default = "station-converter-ja"
}

variable "environment" {
  type    = string
  default = "dev"
}

variable "image" {
  type    = string
  default = "ghcr.io/mt4110/station_converter_ja/api:dev"
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
