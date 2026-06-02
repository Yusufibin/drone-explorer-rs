use common::types::{GpsPosition, MapWaypoint, Sector};
use geojson::{Feature, FeatureCollection, Geometry, Value as GValue};

/// Gère la carte en mémoire (waypoints, secteurs et couverture).
#[derive(Debug, Clone)]
pub struct MapBuilder {
    pub center_lat: f64,
    pub center_lon: f64,
    pub grid_size: f64,
    pub sector_size: f64,
    pub waypoints: Vec<MapWaypoint>,
    pub sectors: Vec<Sector>,
    pub coverage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_html_contains_leaflet_and_geojson() {
        let mut builder = MapBuilder::new(48.8566, 2.3522, 200.0, 100.0);
        builder.add_waypoint(48.8566, 2.3522, "Base", None);

        let html = builder.export_html();

        assert!(html.contains("leaflet"));
        assert!(html.contains("OpenStreetMap"));
        assert!(html.contains("\"type\":\"FeatureCollection\""));
        assert!(html.contains("Couverture"));
    }
}

impl MapBuilder {
    /// Initialise la carte et génère la grille de secteurs.
    pub fn new(center_lat: f64, center_lon: f64, grid_size: f64, sector_size: f64) -> Self {
        let mut builder = Self {
            center_lat,
            center_lon,
            grid_size,
            sector_size,
            waypoints: Vec::new(),
            sectors: Vec::new(),
            coverage: 0.0,
        };
        builder.init_sectors();
        builder
    }

    /// Génère une grille de secteurs en 2D (ex: 5x5) autour du point central.
    pub fn init_sectors(&mut self) {
        let sectors_count = (self.grid_size / self.sector_size).round() as i32;
        let half_count = sectors_count / 2;

        let m_to_lat = 1.0 / 111111.0;
        let m_to_lon = 1.0 / (111111.0 * self.center_lat.to_radians().cos());

        let mut id = 0;

        for i in -half_count..=half_count {
            for j in -half_count..=half_count {
                let offset_lat = (i as f64) * self.sector_size * m_to_lat;
                let offset_lon = (j as f64) * self.sector_size * m_to_lon;

                let lat = self.center_lat + offset_lat;
                let lon = self.center_lon + offset_lon;

                // Calcul de priorité inversement proportionnelle à la distance du centre
                let distance = (offset_lat.powi(2) + offset_lon.powi(2)).sqrt() * 111111.0;
                let priority = if distance == 0.0 {
                    100
                } else {
                    ((500.0 - distance).max(10.0) / 5.0) as u32
                };

                self.sectors.push(Sector {
                    id,
                    center: GpsPosition {
                        lat,
                        lon,
                        alt: 20.0,
                    },
                    priority,
                    reason: format!("Secteur d'exploration {}, distance={:.0}m", id, distance),
                    explored: false,
                });

                id += 1;
            }
        }

        self.recalculate_coverage();
    }

    /// Ajoute un point d'intérêt.
    pub fn add_waypoint(
        &mut self,
        lat: f64,
        lon: f64,
        label: &str,
        photo_path: Option<&str>,
    ) -> bool {
        // Marque aussi le secteur correspondant comme exploré
        self.mark_sector_explored_near(lat, lon);

        self.waypoints.push(MapWaypoint {
            position: GpsPosition {
                lat,
                lon,
                alt: 20.0,
            },
            label: label.to_string(),
            photo_path: photo_path.map(|p| p.to_string()),
            visited: true,
        });

        true
    }

    /// Marque le secteur le plus proche d'un point comme exploré.
    fn mark_sector_explored_near(&mut self, lat: f64, lon: f64) {
        let mut nearest_idx = None;
        let mut min_dist = f64::MAX;

        for (idx, sector) in self.sectors.iter().enumerate() {
            let dist =
                ((sector.center.lat - lat).powi(2) + (sector.center.lon - lon).powi(2)).sqrt();
            if dist < min_dist {
                min_dist = dist;
                nearest_idx = Some(idx);
            }
        }

        if let Some(idx) = nearest_idx {
            // Uniquement si à moins de 80m du centre du secteur
            if min_dist * 111111.0 < 80.0 {
                self.sectors[idx].explored = true;
                self.recalculate_coverage();
            }
        }
    }

    /// Force l'état exploré d'un secteur par son ID.
    pub fn mark_sector_explored(&mut self, sector_id: u32) {
        if let Some(sector) = self.sectors.iter_mut().find(|s| s.id == sector_id) {
            sector.explored = true;
            self.recalculate_coverage();
        }
    }

    /// Recalcule le pourcentage de couverture de la zone.
    fn recalculate_coverage(&mut self) {
        if self.sectors.is_empty() {
            self.coverage = 0.0;
            return;
        }

        let explored_count = self.sectors.iter().filter(|s| s.explored).count();
        self.coverage = (explored_count as f64 / self.sectors.len() as f64) * 100.0;
    }

    /// Récupère le pourcentage global de couverture.
    pub fn get_coverage_percent(&self) -> f64 {
        self.coverage
    }

    /// Exporte la carte au format GeoJSON.
    pub fn export_geojson(&self) -> String {
        let mut features = Vec::new();

        // 1. Ajouter les secteurs sous forme de polygones
        for sector in &self.sectors {
            let m_to_lat = 1.0 / 111111.0;
            let m_to_lon = 1.0 / (111111.0 * sector.center.lat.to_radians().cos());

            let half_w = (self.sector_size / 2.0) * m_to_lon;
            let half_h = (self.sector_size / 2.0) * m_to_lat;

            let poly = vec![vec![
                vec![sector.center.lon - half_w, sector.center.lat - half_h],
                vec![sector.center.lon + half_w, sector.center.lat - half_h],
                vec![sector.center.lon + half_w, sector.center.lat + half_h],
                vec![sector.center.lon - half_w, sector.center.lat + half_h],
                vec![sector.center.lon - half_w, sector.center.lat - half_h],
            ]];

            let geometry = Geometry::new(GValue::Polygon(poly));
            let mut feature = Feature {
                bbox: None,
                geometry: Some(geometry),
                id: Some(geojson::feature::Id::String(format!(
                    "sector-{}",
                    sector.id
                ))),
                properties: None,
                foreign_members: None,
            };

            feature.set_property("id", sector.id);
            feature.set_property("explored", sector.explored);
            feature.set_property("priority", sector.priority);
            feature.set_property("type", "sector");

            features.push(feature);
        }

        // 2. Ajouter les waypoints sous forme de points
        for (i, wp) in self.waypoints.iter().enumerate() {
            let geom = Geometry::new(GValue::Point(vec![wp.position.lon, wp.position.lat]));
            let mut feature = Feature {
                bbox: None,
                geometry: Some(geom),
                id: Some(geojson::feature::Id::String(format!("waypoint-{}", i))),
                properties: None,
                foreign_members: None,
            };

            feature.set_property("label", wp.label.clone());
            feature.set_property("photo_path", wp.photo_path.clone());
            feature.set_property("type", "waypoint");

            features.push(feature);
        }

        let fc = FeatureCollection {
            bbox: None,
            features,
            foreign_members: None,
        };

        fc.to_string()
    }

    /// Exporte une carte HTML autonome Leaflet avec fond OpenStreetMap.
    pub fn export_html(&self) -> String {
        let geojson = self.export_geojson();
        format!(
            r#"<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Drone Explorer Map</title>
  <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css">
  <style>
    html, body, #map {{ height: 100%; margin: 0; }}
    .status {{ position: absolute; z-index: 1000; top: 12px; right: 12px; background: white; padding: 8px 10px; border: 1px solid #999; font: 14px sans-serif; }}
  </style>
</head>
<body>
  <div id="map"></div>
  <div class="status">Couverture: {coverage:.2}% | Secteurs: {sector_count} | Waypoints: {waypoint_count}</div>
  <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
  <script>
    const data = {geojson};
    const map = L.map('map').setView([{lat}, {lon}], 17);
    L.tileLayer('https://tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png', {{
      maxZoom: 20,
      attribution: '&copy; OpenStreetMap contributors'
    }}).addTo(map);
    L.geoJSON(data, {{
      style: feature => feature.properties.type === 'sector'
        ? {{ color: feature.properties.explored ? '#1f8f4d' : '#b42318', weight: 1, fillOpacity: 0.22 }}
        : undefined,
      pointToLayer: (feature, latlng) => L.circleMarker(latlng, {{ radius: 6, color: '#0b5cab', fillOpacity: 0.8 }}),
      onEachFeature: (feature, layer) => {{
        const props = feature.properties || {{}};
        layer.bindPopup(Object.entries(props).map(([k, v]) => `${{k}}: ${{v}}`).join('<br>'));
      }}
    }}).addTo(map);
  </script>
</body>
</html>"#,
            coverage = self.coverage,
            sector_count = self.sectors.len(),
            waypoint_count = self.waypoints.len(),
            lat = self.center_lat,
            lon = self.center_lon,
            geojson = geojson,
        )
    }
}
