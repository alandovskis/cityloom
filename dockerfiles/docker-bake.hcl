# Builds the four toolchain images, the Rust one before the e2e one that starts from it. From the repository root:
#   docker buildx bake -f dockerfiles/docker-bake.hcl            (all four)
#   docker buildx bake -f dockerfiles/docker-bake.hcl docs       (one)

group "default" {
  targets = ["rust", "js", "docs", "e2e"]
}

target "rust" {
  context    = "."
  dockerfile = "dockerfiles/rust.Dockerfile"
  tags       = ["cityloom-rust"]
}

target "js" {
  context    = "dockerfiles"
  dockerfile = "js.Dockerfile"
  tags       = ["cityloom-js"]
}

target "docs" {
  context    = "dockerfiles"
  dockerfile = "docs.Dockerfile"
  tags       = ["cityloom-docs"]
}

target "e2e" {
  context    = "."
  dockerfile = "dockerfiles/e2e.Dockerfile"
  contexts = {
    rust = "target:rust"
  }
  tags = ["cityloom-e2e"]
}
