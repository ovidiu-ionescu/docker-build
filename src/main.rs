use clap::Parser;
use indoc::{indoc, formatdoc};
use serde_derive::Deserialize;
use std::fs::File;
use std::io::prelude::*;

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
#[command(author, version, about, long_about)]
struct CargoCli {
    #[clap(subcommand)]
    docker_build: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    DockerBuild,
}

fn main() {
    CargoCli::parse();
    write_dockerfile();
    write_build_script();
}

fn get_cargo_files() -> Vec<String> {
    let workspace_content = match std::fs::read_to_string("Cargo.toml") {
        Ok(content) => content,
        Err(e) => {
            panic!("Could not read root Cargo.toml. Error: {e}: 「Cargo.toml」");
        }
    };
    match toml::from_str::<CargoWorkspace>(&workspace_content) {
        Ok(cargo_workspace) => cargo_workspace
            .workspace
            .members
            .iter()
            .filter(|member| !member.starts_with("lib"))
            .map(|member| format!("{member}/Cargo.toml"))
            .collect::<Vec<String>>(),
        _ => vec!["Cargo.toml".to_string()],
    }
}

fn write_dockerfile() {
    let dockerfile = "Dockerfile";
    println!("Generating {dockerfile}");
    let images = get_cargo_files()
        .iter()
        .filter_map(|file| docker_image_section(file).ok())
        .collect::<Vec<String>>();

    let preamble = indoc! {r#"
        # This file is generated by docker-build
        # DO NOT EDIT BY HAND;
        # Edit docker-build instead.
        FROM rust:slim AS builder
        WORKDIR /usr/src/myapp
        # maybe use --link
        COPY . .
        LABEL stage="builder"
        RUN cargo build --release
        "#};
    let mut file = File::create(dockerfile).unwrap();
    file.write_all(preamble.as_bytes()).unwrap();
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
            eprintln!("Error reading the config file: {e} 「{file_name}」");
            return Err(e.to_string());
        }
    };
    let config: Component = match toml::from_str(&file_content) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Can not parse toml file: {e} 「{file_name}」");
            return Err(e.to_string());
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
        let err = format!("Error: At least one author is needed, in 「{file_name}」");
        eprintln!("{err}");
        return Err(err);
    }
    let author = authors[0].clone();

    Ok(formatdoc! { r#"
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

        "#})
}

const BUILD_SCRIPT: &str = "build_docker.sh";

fn write_build_script() {
    println!("Generating {BUILD_SCRIPT}");
    let packages = get_cargo_files()
        .iter()
        .filter_map(|file| get_config(file).ok())
        .collect::<Vec<Package>>();
    build_script(&packages);
}

fn build_script(packages: &[Package]) {
    let mut file = File::create(BUILD_SCRIPT).unwrap();
    let preamble = indoc! { r#"
        #!/usr/bin/env bash
        # This file is generated by docker-build
        # DO NOT EDIT BY HAND;
        # Edit docker-build instead.

        set -e

        eval $(minikube docker-env)

        docker build .
        "#};
    file.write_all(preamble.as_bytes()).unwrap();

    packages.iter().for_each(|package| {
        let name = &package.name;
        let version = &package.version;
        let image = formatdoc! { r#"

            docker tag $(docker image ls --filter "label=tag={name}:v{version}" -q) {name}:v{version}
            "#,
        };
        file.write_all(image.as_bytes()).unwrap();
    });

    let end = indoc! { r#"

        docker image prune --filter label=stage=builder -f
        "#};
    file.write_all(end.as_bytes()).unwrap();
}

