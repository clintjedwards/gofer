//! Generates the API reference section of the documentation book from the OpenAPI spec.
//!
//! Reads `docs/src/assets/openapi.json` and writes one markdown page per API tag plus a schemas
//! page into `docs/src/api/`. It also rewrites the generated block in `docs/src/SUMMARY.md` so
//! new tags show up in the sidebar without manual edits. Run via `make generate-api-docs`.
//!
//! The generator only handles the OpenAPI subset that dropshot/schemars actually emit (allOf
//! wrappers around $refs, oneOf of string enums and externally tagged data variants, nullable, maps via
//! additionalProperties). If the spec grows a shape it doesn't know, it falls back to a generic
//! label rather than failing the build.

use serde_json::Value;
use std::fmt::Write;

const SPEC_PATH: &str = "docs/src/assets/openapi.json";
const OUT_DIR: &str = "docs/src/api";
const SUMMARY_PATH: &str = "docs/src/SUMMARY.md";
const SUMMARY_START: &str = "<!-- API_DOCS_GEN_START -->";
const SUMMARY_END: &str = "<!-- API_DOCS_GEN_END -->";

/// Order operations the way you'd read them, not alphabetically by verb.
const METHOD_ORDER: [&str; 7] = ["get", "post", "put", "patch", "delete", "head", "options"];

struct Operation {
    path: String,
    method: String,
    op: Value,
}

fn main() {
    let spec: Value = serde_json::from_str(
        &std::fs::read_to_string(SPEC_PATH).expect("could not read openapi spec"),
    )
    .expect("could not parse openapi spec");

    std::fs::create_dir_all(OUT_DIR).expect("could not create api docs dir");

    // Tags come from the spec in their declared order; operations get grouped underneath them.
    let tags: Vec<(String, String)> = spec["tags"]
        .as_array()
        .expect("spec has no tags")
        .iter()
        .map(|t| {
            (
                t["name"].as_str().unwrap().to_string(),
                t["description"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();

    let mut ops_by_tag: std::collections::HashMap<String, Vec<Operation>> =
        std::collections::HashMap::new();

    // The spec file keeps paths in sorted order, so related endpoints naturally group together
    // on each page.
    for (path, item) in spec["paths"].as_object().unwrap() {
        for method in METHOD_ORDER {
            let Some(op) = item.get(method) else {
                continue;
            };
            let tag = op["tags"][0]
                .as_str()
                .unwrap_or_else(|| panic!("operation {method} {path} has no tag"));
            ops_by_tag
                .entry(tag.to_string())
                .or_default()
                .push(Operation {
                    path: path.clone(),
                    method: method.to_string(),
                    op: op.clone(),
                });
        }
    }

    let version = spec["info"]["version"].as_str().unwrap_or("unknown");

    write_index(&tags, version);
    for (tag, desc) in &tags {
        let ops = ops_by_tag
            .remove(tag)
            .unwrap_or_else(|| panic!("tag {tag} has no operations"));
        write_tag_page(tag, desc, &ops);
    }
    if !ops_by_tag.is_empty() {
        let missing: Vec<&String> = ops_by_tag.keys().collect();
        panic!("operations reference tags missing from the spec's tag list: {missing:?}");
    }
    write_schemas_page(&spec);
    update_summary(&tags);

    println!("Wrote {} api reference pages to {OUT_DIR}", tags.len() + 2);
}

fn slug(tag: &str) -> String {
    tag.to_lowercase().replace(' ', "_")
}

/// Mirrors mdbook's heading id algorithm so links to generated headings hold up under
/// mdbook-linkcheck. Lowercases, keeps alphanumerics plus `_` and `-`, turns whitespace into `-`,
/// and drops everything else.
fn anchor(text: &str) -> String {
    text.chars()
        .filter_map(|ch| {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

/// Collapses whitespace runs. Some descriptions in the spec come from indented Rust string
/// literals and carry big runs of spaces.
fn normalize_ws(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Makes text safe for a markdown table cell.
fn cell(text: &str) -> String {
    normalize_ws(text).replace('|', "\\|")
}

/// Renders a schema description as standalone markdown. These come from rustdoc comments, and a
/// few (the SDK's Pipeline and Task) carry rustdoc sections with Rust usage examples. Those
/// examples are for SDK users rather than API callers, and schemars has already joined their
/// lines together so the code is unreadable anyway; drop everything from the first heading or
/// code fence onward.
fn description_block(text: &str) -> String {
    // Intra-doc links like [`Pipeline`] have no URL here; keep just the code span.
    let text = text.replace("[`", "`").replace("`]", "`");

    let mut paragraphs = Vec::new();
    for paragraph in text.split("\n\n") {
        let paragraph = normalize_ws(paragraph);
        if paragraph.starts_with('#') || paragraph.starts_with("```") {
            break;
        }
        if !paragraph.is_empty() {
            paragraphs.push(paragraph);
        }
    }
    paragraphs.join("\n\n")
}

fn ref_name(reference: &str) -> &str {
    reference.rsplit('/').next().unwrap()
}

fn schema_link(name: &str, prefix: &str) -> String {
    format!("[{name}]({prefix}#{})", anchor(name))
}

/// Renders a schema as a short human-readable type, linking $refs to the schemas page.
/// `prefix` is the relative path to schemas.md, empty when already on that page.
fn render_type(schema: &Value, prefix: &str) -> String {
    let base = render_type_inner(schema, prefix);
    if schema["nullable"].as_bool().unwrap_or(false) {
        format!("{base} (nullable)")
    } else {
        base
    }
}

fn render_type_inner(schema: &Value, prefix: &str) -> String {
    if let Some(reference) = schema["$ref"].as_str() {
        return schema_link(ref_name(reference), prefix);
    }

    // schemars wraps a $ref in a single-element allOf when the field carries its own description.
    if let Some(all_of) = schema["allOf"].as_array()
        && all_of.len() == 1
    {
        return render_type_inner(&all_of[0], prefix);
    }

    match schema["type"].as_str() {
        Some("array") => {
            let items = schema
                .get("items")
                .map(|i| render_type_inner(i, prefix))
                .unwrap_or_else(|| "any".to_string());
            format!("array of {items}")
        }
        Some("object") => match schema.get("additionalProperties") {
            Some(extra) if extra.is_object() && !extra.as_object().unwrap().is_empty() => {
                format!("map of string to {}", render_type_inner(extra, prefix))
            }
            Some(_) => "map of string to any".to_string(),
            None => "object".to_string(),
        },
        Some("string") => match schema["enum"].as_array() {
            Some(values) => {
                let values: Vec<String> = values
                    .iter()
                    .map(|v| format!("`{}`", v.as_str().unwrap_or_default()))
                    .collect();
                format!("one of {}", values.join(", "))
            }
            None => "string".to_string(),
        },
        Some(other) => other.to_string(),
        None => "any".to_string(),
    }
}

fn is_websocket(op: &Value) -> bool {
    op.get("x-dropshot-websocket").is_some()
}

fn method_badge(method: &str, websocket: bool) -> String {
    let label = if websocket {
        "WS".to_string()
    } else {
        method.to_uppercase()
    };
    let class = if websocket { "ws" } else { method };
    format!("<span class=\"api-method api-method-{class}\">{label}</span>")
}

/// Heading text for an operation: the summary without its trailing period, or the operation id
/// when there's no summary.
fn op_heading(op: &Value) -> String {
    match op["summary"].as_str() {
        Some(summary) => summary.trim().trim_end_matches('.').to_string(),
        None => op["operationId"].as_str().unwrap_or("unnamed").to_string(),
    }
}

fn write_index(tags: &[(String, String)], version: &str) {
    let mut out = String::new();

    writeln!(out, "# API Reference\n").unwrap();
    writeln!(
        out,
        "Gofer exposes a REST API; every endpoint lives under `/api` and speaks JSON unless \
         noted otherwise. These pages are generated from the \
         [OpenAPI spec](../assets/openapi.json) (Gofer v{version}) by `make generate-api-docs`.\n"
    )
    .unwrap();

    writeln!(out, "## Authentication\n").unwrap();
    writeln!(
        out,
        "Every request needs a bearer token. See \
         [Authentication and Authorization](../ref/server_configuration/authz_n.md) for how \
         tokens are created and what they can do.\n"
    )
    .unwrap();

    writeln!(out, "## Versioning\n").unwrap();
    writeln!(
        out,
        "Every request must carry a `gofer-api-version` header. The only version right now is \
         `v0`.\n"
    )
    .unwrap();

    writeln!(out, "```bash").unwrap();
    writeln!(out, "curl http://localhost:8080/api/system/metadata \\").unwrap();
    writeln!(out, "  -H \"Authorization: Bearer $GOFER_TOKEN\" \\").unwrap();
    writeln!(out, "  -H \"gofer-api-version: v0\"").unwrap();
    writeln!(out, "```\n").unwrap();

    writeln!(out, "## Websockets\n").unwrap();
    writeln!(
        out,
        "A few endpoints (event streaming, log following, task attach) upgrade the connection \
         to a websocket instead of returning a JSON body. They're marked with a \
         <span class=\"api-method api-method-ws\">WS</span> badge.\n"
    )
    .unwrap();

    writeln!(out, "## Sections\n").unwrap();
    writeln!(out, "| Section | Description |").unwrap();
    writeln!(out, "| ------- | ----------- |").unwrap();
    for (tag, desc) in tags {
        writeln!(out, "| [{tag}](./{}.md) | {} |", slug(tag), cell(desc)).unwrap();
    }
    writeln!(
        out,
        "| [Schemas](./schemas.md) | Request and response object definitions. |"
    )
    .unwrap();

    std::fs::write(format!("{OUT_DIR}/README.md"), out).expect("could not write api index page");
}

fn write_tag_page(tag: &str, description: &str, ops: &[Operation]) {
    let mut out = String::new();

    writeln!(out, "# {tag}\n").unwrap();
    if !description.is_empty() {
        writeln!(out, "{}\n", normalize_ws(description)).unwrap();
    }

    // A small index up top so you can see a tag's surface area at a glance. Anchors are built
    // from the same headings mdbook will generate below; a duplicate summary would make them
    // ambiguous, so fail loudly instead of emitting broken links.
    let mut seen = std::collections::HashSet::new();
    for op in ops {
        let heading = op_heading(&op.op);
        if !seen.insert(anchor(&heading)) {
            panic!("duplicate operation summary \"{heading}\" on tag {tag}");
        }
    }

    writeln!(out, "| Method | Endpoint |").unwrap();
    writeln!(out, "| ------ | -------- |").unwrap();
    for op in ops {
        writeln!(
            out,
            "| {} | [`{}`](#{}) |",
            method_badge(&op.method, is_websocket(&op.op)),
            op.path,
            anchor(&op_heading(&op.op)),
        )
        .unwrap();
    }

    for op in ops {
        writeln!(out, "\n---\n").unwrap();
        write_operation(&mut out, op);
    }

    std::fs::write(format!("{OUT_DIR}/{}.md", slug(tag)), out)
        .unwrap_or_else(|_| panic!("could not write page for tag {tag}"));
}

fn write_operation(out: &mut String, operation: &Operation) {
    let op = &operation.op;
    let websocket = is_websocket(op);

    writeln!(out, "## {}\n", op_heading(op)).unwrap();
    writeln!(
        out,
        "<div class=\"api-endpoint\">{}<code>{}</code></div>\n",
        method_badge(&operation.method, websocket),
        operation.path,
    )
    .unwrap();

    if let Some(description) = op["description"].as_str() {
        writeln!(out, "{}\n", description.trim()).unwrap();
    }

    if websocket {
        writeln!(
            out,
            "This endpoint upgrades the connection to a websocket: after the `101` response the \
             server streams data over the socket instead of returning a body.\n"
        )
        .unwrap();
    }

    let params = op["parameters"].as_array().cloned().unwrap_or_default();
    for (location, title) in [
        ("path", "Path parameters"),
        ("query", "Query parameters"),
        ("header", "Header parameters"),
    ] {
        let group: Vec<&Value> = params
            .iter()
            .filter(|p| p["in"].as_str() == Some(location))
            .collect();
        if group.is_empty() {
            continue;
        }

        writeln!(out, "**{title}**\n").unwrap();
        writeln!(out, "| Name | Type | Required | Description |").unwrap();
        writeln!(out, "| ---- | ---- | -------- | ----------- |").unwrap();
        for param in group {
            // Nullability on a query param just means optional, which the Required column
            // already covers, so render the inner type.
            writeln!(
                out,
                "| `{}` | {} | {} | {} |",
                param["name"].as_str().unwrap_or_default(),
                render_type_inner(&param["schema"], "./schemas.md"),
                if param["required"].as_bool().unwrap_or(false) {
                    "yes"
                } else {
                    "no"
                },
                cell(param["description"].as_str().unwrap_or_default()),
            )
            .unwrap();
        }
        writeln!(out).unwrap();
    }

    if let Some(body) = op.get("requestBody") {
        let required = if body["required"].as_bool().unwrap_or(false) {
            "required"
        } else {
            "optional"
        };
        let rendered = match body["content"].as_object() {
            Some(content) if content.contains_key("application/json") => {
                render_type(&content["application/json"]["schema"], "./schemas.md")
            }
            Some(content) if content.contains_key("application/octet-stream") => {
                "raw bytes (`application/octet-stream`)".to_string()
            }
            _ => "unknown".to_string(),
        };
        writeln!(out, "**Request body** ({required}): {rendered}\n").unwrap();
    }

    if let Some(responses) = op["responses"].as_object() {
        writeln!(out, "**Responses**\n").unwrap();
        writeln!(out, "| Status | Body | Description |").unwrap();
        writeln!(out, "| ------ | ---- | ----------- |").unwrap();
        for (status, response) in responses {
            // The shared 4XX/5XX responses are $refs into components/responses; all of them are
            // the standard Error shape.
            let (description, body) = if response.get("$ref").is_some() {
                ("Error".to_string(), schema_link("Error", "./schemas.md"))
            } else {
                let body = match response["content"].as_object() {
                    Some(content) if content.contains_key("application/json") => {
                        render_type(&content["application/json"]["schema"], "./schemas.md")
                    }
                    Some(_) => "raw bytes".to_string(),
                    None => "none".to_string(),
                };
                (
                    response["description"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    body,
                )
            };
            writeln!(out, "| `{status}` | {body} | {} |", cell(&description)).unwrap();
        }
        writeln!(out).unwrap();
    }
}

fn write_schemas_page(spec: &Value) {
    let mut out = String::new();

    writeln!(out, "# Schemas\n").unwrap();
    writeln!(
        out,
        "Object definitions used by request and response bodies. Field types link to other \
         schemas on this page.\n"
    )
    .unwrap();

    for (name, schema) in spec["components"]["schemas"].as_object().unwrap() {
        writeln!(out, "## {name}\n").unwrap();

        if let Some(description) = schema["description"].as_str() {
            writeln!(out, "{}\n", description_block(description)).unwrap();
        }

        if let Some(variants) = schema["oneOf"].as_array() {
            write_enum(&mut out, variants);
            continue;
        }

        match schema["type"].as_str() {
            Some("object") => {
                let Some(properties) = schema["properties"].as_object() else {
                    writeln!(out, "An object with no defined fields.\n").unwrap();
                    continue;
                };
                let required: std::collections::HashSet<&str> = schema["required"]
                    .as_array()
                    .map(|r| r.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();

                writeln!(out, "| Field | Type | Required | Description |").unwrap();
                writeln!(out, "| ----- | ---- | -------- | ----------- |").unwrap();
                for (field, field_schema) in properties {
                    writeln!(
                        out,
                        "| `{field}` | {} | {} | {} |",
                        render_type(field_schema, ""),
                        if required.contains(field.as_str()) {
                            "yes"
                        } else {
                            "no"
                        },
                        cell(field_schema["description"].as_str().unwrap_or_default()),
                    )
                    .unwrap();
                }
                writeln!(out).unwrap();
            }
            Some("string") if schema["enum"].is_array() => {
                // Same table as a oneOf enum. The schema's description was already written
                // above, so keep it out of the rows.
                let mut values = schema.clone();
                values["description"] = Value::Null;
                write_enum(&mut out, &[values]);
            }
            Some("string") => {
                writeln!(out, "A {}.\n", render_type_inner(schema, "")).unwrap();
            }
            other => {
                writeln!(out, "A {}.\n", other.unwrap_or("value")).unwrap();
            }
        }
    }

    std::fs::write(format!("{OUT_DIR}/schemas.md"), out).expect("could not write schemas page");
}

/// Renders a Rust enum. Unit variants come through as string enums (sometimes several values
/// share one variant schema); variants with data are serde's externally tagged form, an object
/// with a single key naming the variant.
fn write_enum(out: &mut String, variants: &[Value]) {
    let has_data = variants
        .iter()
        .any(|v| v["type"].as_str() == Some("object"));

    if has_data {
        writeln!(
            out,
            "One of the variants below. Variants without a payload are sent as a plain string; \
             the rest are an object with the variant name as its only key.\n"
        )
        .unwrap();
        writeln!(out, "| Variant | Payload | Description |").unwrap();
        writeln!(out, "| ------- | ------- | ----------- |").unwrap();
    } else {
        writeln!(out, "A string, one of:\n").unwrap();
        writeln!(out, "| Value | Description |").unwrap();
        writeln!(out, "| ----- | ----------- |").unwrap();
    }

    for variant in variants {
        let description = cell(variant["description"].as_str().unwrap_or_default());

        if let Some(values) = variant["enum"].as_array() {
            for value in values {
                let value = value.as_str().unwrap_or_default();
                if has_data {
                    writeln!(out, "| `{value}` | none | {description} |").unwrap();
                } else {
                    writeln!(out, "| `{value}` | {description} |").unwrap();
                }
            }
            continue;
        }

        let Some(properties) = variant["properties"].as_object() else {
            writeln!(out, "| {} | | {description} |", render_type(variant, "")).unwrap();
            continue;
        };
        for (name, payload) in properties {
            writeln!(
                out,
                "| `{name}` | {} | {description} |",
                render_payload(payload)
            )
            .unwrap();
        }
    }
    writeln!(out).unwrap();
}

/// Inline summary of a data variant's payload, e.g. `` `namespace_id`: string, `run_id`: integer ``.
fn render_payload(payload: &Value) -> String {
    let Some(fields) = payload["properties"].as_object() else {
        return render_type(payload, "");
    };
    let required: std::collections::HashSet<&str> = payload["required"]
        .as_array()
        .map(|r| r.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let fields: Vec<String> = fields
        .iter()
        .map(|(name, schema)| {
            let optional = if required.contains(name.as_str()) {
                ""
            } else {
                " (optional)"
            };
            format!("`{name}`: {}{optional}", render_type(schema, ""))
        })
        .collect();
    if fields.is_empty() {
        "none".to_string()
    } else {
        fields.join("<br>")
    }
}

fn update_summary(tags: &[(String, String)]) {
    let summary = std::fs::read_to_string(SUMMARY_PATH).expect("could not read SUMMARY.md");

    let start = summary
        .find(SUMMARY_START)
        .expect("SUMMARY.md is missing the API_DOCS_GEN_START marker")
        + SUMMARY_START.len();
    let end = summary
        .find(SUMMARY_END)
        .expect("SUMMARY.md is missing the API_DOCS_GEN_END marker");

    let mut block = String::from("\n");
    block.push_str("- [API Reference](./api/README.md)\n");
    for (tag, _) in tags {
        writeln!(block, "  - [{tag}](./api/{}.md)", slug(tag)).unwrap();
    }
    block.push_str("  - [Schemas](./api/schemas.md)\n");

    let updated = format!("{}{}{}", &summary[..start], block, &summary[end..]);
    std::fs::write(SUMMARY_PATH, updated).expect("could not write SUMMARY.md");
}
