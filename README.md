## Scraper und Renderer für den Karlsruher Abfallkalender

Rust installieren, dann

1. Sperrmülldaten scrapen mit `cargo r --release fetch dates.json`
2. Karte saugen mit `wget https://download.geofabrik.de/europe/germany/baden-wuerttemberg/karlsruhe-regbez-latest.osm.pbf`
3. Kartendaten verarbeiten mit `cargo r --release process data.json karlsruhe-regbez-latest.osm.pbf processed.json`
4. Kartendaten rendern mit `cargo r --release render processed.json > index.html`

Viel Spaß 👋
