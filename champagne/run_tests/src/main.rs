use gofer_sdk::config::{Pipeline, Task};

fn main() {
    Pipeline::new("run-tests", "Run Project Tests")
        .description("Runs all cargo tests for the overall Gofer workspace.")
        .tasks(vec![Task::new(
            "run-cargo-test",
            "ghcr.io/clintjedwards/gofer/tools:rust",
        )
            .description("Run cargo test command for workspace")
            .always_pull_newest_image(true)
            .inject_api_token(true)
            // We need to insert the gofer api base url so that the gofer CLI knows where to send requests.
            .variables(vec![("GOFER_API_BASE_URL", "http://172.17.0.1:8080")])
            .script(
                r#"
                set -euxo pipefail

                # If this isn't set by the Github extension then set it ourselves to the main branch
                GOFER_EXTENSION_GITHUB_PULLREQUEST_BRANCH="${GOFER_EXTENSION_GITHUB_PULLREQUEST_BRANCH:-main}"
            
                git clone --depth 1 --branch "$GOFER_EXTENSION_GITHUB_PULLREQUEST_BRANCH" https://github.com/clintjedwards/gofer --single-branch

                cd gofer

                # === Restore Cache ===
                if gofer pipeline object get run-tests cache > /tmp/rust_cache.tar.gz; then
                    if tar -xzf /tmp/rust_cache.tar.gz -C /gofer; then
                        rm -rf /root/.cargo
                        mv .cargo /root
                        echo "✅ Cache restored"
                    else
                        echo "⚠️ Cache extraction failed, proceeding without cache."
                    fi
                else
                    echo "⚠️ Cache fetch failed, proceeding without cache."
                fi

                # === Run Tests ===
                cargo test --color=always

                # === Save Cache ===
                tar -czf /tmp/rust_cache.tar.gz -C /gofer target -C /root .cargo

                gofer pipeline object put run-tests --force cache /tmp/rust_cache.tar.gz
                "#,
            )])
        .finish()
        .unwrap();
}
