/// Embed swagger.yaml at compile time – path is relative to the crate root.
static SWAGGER_YAML: &str = include_str!("../../swagger.yaml");

/// Serves the raw OpenAPI YAML spec at GET /docs/openapi.yaml
pub async fn get_swagger_yaml() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok()
        .content_type("application/yaml")
        .body(SWAGGER_YAML)
}

/// Serves an HTML page that loads Swagger UI from CDN and points it at
/// the local /docs/openapi.yaml endpoint.
pub async fn get_swagger_ui() -> actix_web::HttpResponse {
    let html = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Transaction API – Swagger UI</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
  <style>
    body { margin: 0; background: #1a1a2e; }
    .swagger-ui .topbar { background: #16213e; }
    .swagger-ui .topbar .download-url-wrapper { display: none; }
  </style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    SwaggerUIBundle({
      url: "/docs/openapi.yaml",
      dom_id: "#swagger-ui",
      presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
      layout: "BaseLayout",
      deepLinking: true,
    });
  </script>
</body>
</html>"##;

    actix_web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}
