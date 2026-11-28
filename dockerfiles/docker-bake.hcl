# Builds the four toolchain images, the Rust one before the e2e one that starts from it. From the repository root:
#   docker buildx bake -f dockerfiles/docker-bake.hcl            (all four)
#   docker buildx bake -f dockerfiles/docker-bake.hcl docs       (one)
# CI (.github/workflows/images.yml) sets REGISTRY and TAG to push them to GHCR; without them the images are the local
# cityloom-<name>:latest that the README's commands run.

variable "REGISTRY" {
  default = ""
}

variable "TAG" {
  default = "latest"
}

group "default" {
  targets = ["rust", "js", "docs", "e2e"]
}

# What every image carries: the repository it is built from, which links a GHCR package to it.
target "_common" {
  labels = {
    "org.opencontainers.image.source" = "https://github.com/alandovskis/cityloom"
  }
}

target "rust" {
  inherits   = ["_common"]
  context    = "."
  dockerfile = "dockerfiles/rust.Dockerfile"
  tags       = ["${REGISTRY}cityloom-rust:${TAG}"]
}

target "js" {
  inherits   = ["_common"]
  context    = "dockerfiles"
  dockerfile = "js.Dockerfile"
  tags       = ["${REGISTRY}cityloom-js:${TAG}"]
}

target "docs" {
  inherits   = ["_common"]
  context    = "dockerfiles"
  dockerfile = "docs.Dockerfile"
  tags       = ["${REGISTRY}cityloom-docs:${TAG}"]
}

target "e2e" {
  inherits   = ["_common"]
  context    = "."
  dockerfile = "dockerfiles/e2e.Dockerfile"
  contexts = {
    rust = "target:rust"
  }
  tags = ["${REGISTRY}cityloom-e2e:${TAG}"]
}
