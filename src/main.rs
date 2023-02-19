use clap::Parser;
use serde_derive::Deserialize;
use std::fs::File;
use std::io::prelude::*;

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    DockerBuild(DockerBuildArgs),
}

#[derive(clap::Args)]
#[command(author, version, about, long_about)]
struct DockerBuildArgs {}

fn main() {
    CargoCli::parse();
    write_dockerfile();
    write_build_script();
}

fn get_cargo_files() -> Vec<String> {
    let workspace_content = match std::fs::read_to_string("Cargo.toml") {
        Ok(content) => content,
        Err(e) => {
            panic!("Error: {e}: 「Cargo.toml」");
        }
    };
    let cargo: CargoWorkspace = toml::from_str(&workspace_content).unwrap();
    cargo
        .workspace
        .members
        .iter()
        .filter(|member| !member.starts_with("lib"))
        .map(|member| format!("{member}/Cargo.toml"))
        .collect::<Vec<String>>()
}

fn write_dockerfile() {
    let images = get_cargo_files()
        .iter()
        .map(|file| docker_image_section(file).unwrap())
        .collect::<Vec<String>>();

    let preable = r#"
# This file is generated by docker-install
# DO NOT EDIT BY HAND;
# Edit docker-install instead.
FROM rust:slim AS builder
WORKDIR /usr/src/myapp
# maybe use --link
COPY . .
LABEL stage="builder"
RUN cargo build --release
"#;
    let mut file = File::create("Dockerfile").unwrap();
    file.write_all(preable.as_bytes()).unwrap();
    for image in images {
        file.write_all(image.as_bytes()).unwrap();
    }
}

#[derive(Debug, Deserialize)]
struct Package {
    name:        String,
    version:     String,
    authors:     Vec<String>,
    description: String,
    repository:  String,
}

#[derive(Debug, Deserialize)]
struct Component {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Workspace {
    members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CargoWorkspace {
    workspace: Workspace,
}

fn get_config(file_name: &str) -> Result<Package, String> {
    let file_content = match std::fs::read_to_string(file_name) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error: {e} 「{file_name}」");
            return Err(e.to_string());
        }
    };
    let config: Component = match toml::from_str(&file_content) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error: {e} 「{file_name}」");
            panic!("{}", e.to_string());
        }
    };
    Ok(config.package)
}
fn docker_image_section(file_name: &str) -> Result<String, String> {
    let package = get_config(file_name)?;
    let name = package.name;
    let version = package.version;
    let authors = package.authors;
    let description = package.description.trim();
    let repository = package.repository;
    if authors.is_empty() {
        panic!("At least one author is needed, in 「{file_name}」");
    }
    let author = authors[0].clone();

    Ok(format!(
        r#"
### {name}
FROM debian:bullseye-slim
COPY --from=builder /usr/src/myapp/target/release/{name} /usr/local/bin/{name}
MAINTAINER {author}
LABEL maintainer="{author}" \
  version="{version}" \
  tag="{name}:v{version}" \
  description="{description}" \
  repository="{repository}" \
  name="{name}"\
  app="{name}"

#ENTRYPOINT ["/usr/local/bin/{name}"]
EXPOSE 8080/tcp 8081/tcp

CMD ["{name}"]

"#,
    ))
}

fn write_build_script() {
    let packages = get_cargo_files()
        .iter()
        .map(|file| get_config(file).unwrap())
        .collect::<Vec<Package>>();
    build_script(&packages);
}

fn build_script(packages: &[Package]) {
    let mut file = File::create("build_docker.sh").unwrap();
    let preable = r#"#!/usr/bin/env bash
# This file is generated by the build.rs
# DO NOT EDIT BY HAND;
# Edit the build.rs instead.

set -e

eval $(minikube docker-env)

docker build .
"#;
    file.write_all(preable.as_bytes()).unwrap();

    packages.iter().for_each(|package| {
        let name = &package.name;
        let version = &package.version;
        let image = format!(
            r#"
docker tag $(docker image ls --filter "label=tag={name}:v{version}" -q) {name}:v{version}"#
        );
        file.write_all(image.as_bytes()).unwrap();
    });

    let end = r#"

docker image prune --filter label=stage=builder -f
"#;
    file.write_all(end.as_bytes()).unwrap();
}
