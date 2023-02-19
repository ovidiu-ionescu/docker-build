# docker-build

Helps building the executables in a Rust workspace and deploy them inside minikube

It generates two files:
- Dockerfile
- build_docker.sh

When ran from the root of a workspace project, the program will read the `Cargo.toml` 
files from every subproject and create a multi stage `Dockerfile`, that can compile the 
sources and produce the images.

The images will have labels with information extracted from `Cargo.toml`

A shell file `build_docker.sh` will be generated. 
When run, it will invoke the Dockerfile creation
and will tag the newly created images using the labels in the images.

The program will only do the file generation, it will not run anything so
you need to run the `build_docker.sh` manually.

## Installation
This program is meant to run as a Cargo subcommand. To install it from source, run:
```bash
cargo install --path .
```
