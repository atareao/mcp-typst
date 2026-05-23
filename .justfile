user     := "atareao"
name     := `basename ${PWD}`
version  := `vampus show`
http_port := "3007"

list:
    @just --list

# ── Rust ──────────────────────────────────────────────────────────────────────

build:
    cargo build

release:
    cargo build --release

run:
    cargo run

test:
    cargo test

lint:
    cargo clippy --all-targets --all-features

fmt:
    cargo fmt -- --check

fmt-fix:
    cargo fmt

# ── Docker ────────────────────────────────────────────────────────────────────

docker-build:
    @podman build \
        --tag={{user}}/{{name}}:{{version}} \
        --tag={{user}}/{{name}}:latest .

docker-run-http:
    @podman run -d --rm --name {{name}} \
        -e MCP_TRANSPORT=http \
        -e MCP_HOST=0.0.0.0 \
        -e MCP_PORT={{http_port}} \
        -p {{http_port}}:{{http_port}} \
        {{user}}/{{name}}:latest && \
    sleep 2 && \
    echo "Server ready on http://127.0.0.1:{{http_port}}"

docker-run-stdio:
    @podman run -i --rm \
        {{user}}/{{name}}:latest

docker-stop:
    @podman stop {{name}} 2>/dev/null || true

# ── HTTP Testing ──────────────────────────────────────────────────────────────

http-list-tools:
    podman stop {{name}} 2>/dev/null || true
    podman run -d --rm --name {{name}} \
      -e MCP_TRANSPORT=http \
      -e MCP_HOST=0.0.0.0 \
      -e MCP_PORT={{http_port}} \
      -p {{http_port}}:{{http_port}} \
      {{user}}/{{name}}:latest
    @sleep 2
    @scripts/http-test.sh {{http_port}} tools/list | sed 's/^data: //' | python3 -m json.tool

http-fonts:
    podman stop {{name}} 2>/dev/null || true
    podman run -d --rm --name {{name}} \
      -e MCP_TRANSPORT=http \
      -e MCP_HOST=0.0.0.0 \
      -e MCP_PORT={{http_port}} \
      -p {{http_port}}:{{http_port}} \
      {{user}}/{{name}}:latest
    @sleep 2
    @scripts/http-test.sh {{http_port}} list_fonts | sed 's/^data: //' | python3 -c 'import sys, json; data=json.loads(sys.stdin.read()); [print(b.get("text","")) for b in data["result"]["content"]]'

http-compile file:
    podman stop {{name}} 2>/dev/null || true
    podman run -d --rm --name {{name}} \
      -e MCP_TRANSPORT=http \
      -e MCP_HOST=0.0.0.0 \
      -e MCP_PORT={{http_port}} \
      -p {{http_port}}:{{http_port}} \
      {{user}}/{{name}}:latest
    @sleep 2
    @scripts/http-test.sh {{http_port}} compile "file={{file}}" "embed=true" | sed 's/^data: //' | python3 -c 'import sys, json; data=json.loads(sys.stdin.read()); [print(b.get("text","")) for b in data["result"]["content"]]'

# ── Push ──────────────────────────────────────────────────────────────────────

push:
    @podman image push {{user}}/{{name}}:{{version}}
    @podman image push {{user}}/{{name}}:latest

upgrade:
    #!/bin/fish
    vampus upgrade --patch
    set VERSION $(vampus show)
    cargo update
    git commit -am "Upgrade to version $VERSION"
    git tag -a "$VERSION" -m "Version $VERSION"
    podman image list | grep {{name}} | sort -r | tail -n +5 | awk '{print $3}' | while read id; echo $id; podman rmi $id; end
    just docker-build push
