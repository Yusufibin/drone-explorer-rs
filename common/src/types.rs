//! Types partagés pour le projet drone-explorer.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Position GPS en 3D.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GpsPosition {
    pub lat: f64,
    pub lon: f64,
    pub alt: f64,
}

impl fmt::Display for GpsPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.6}, {:.6}, {:.1}m)", self.lat, self.lon, self.alt)
    }
}

/// Mode de vol du drone (compatible ArduPilot).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FlightMode {
    Manual,
    Stabilize,
    AltHold,
    Loiter,
    Guided,
    Auto,
    Rtl,
    Land,
    Brake,
    Unknown,
}

impl fmt::Display for FlightMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Manual => "MANUAL",
            Self::Stabilize => "STABILIZE",
            Self::AltHold => "ALT_HOLD",
            Self::Loiter => "LOITER",
            Self::Guided => "GUIDED",
            Self::Auto => "AUTO",
            Self::Rtl => "RTL",
            Self::Land => "LAND",
            Self::Brake => "BRAKE",
            Self::Unknown => "UNKNOWN",
        };
        write!(f, "{}", name)
    }
}

/// Télémétrie en temps réel du drone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Telemetry {
    pub position: GpsPosition,
    pub heading: f64,
    pub speed: f64,
    pub mode: FlightMode,
    pub armed: bool,
    pub timestamp: DateTime<Utc>,
}

/// Statut de la batterie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryStatus {
    pub percent: f32,
    pub voltage: f32,
    pub remaining_minutes: Option<f32>,
}

/// Boîte englobante d'une détection visuelle (YOLO).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Objet détecté par le système de vision (YOLO).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedObject {
    pub class_name: String,
    pub confidence: f32,
    pub bbox: BoundingBox,
    pub position: Option<GpsPosition>,
}

/// Classification du type de terrain survolé.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TerrainType {
    Urban,
    Vegetation,
    Water,
    Road,
    Desert,
    Forest,
    Unknown,
}

impl fmt::Display for TerrainType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Urban => "Urbain",
            Self::Vegetation => "Végétation",
            Self::Water => "Eau",
            Self::Road => "Route",
            Self::Desert => "Désert",
            Self::Forest => "Forêt",
            Self::Unknown => "Inconnu",
        };
        write!(f, "{}", name)
    }
}

/// Conditions météorologiques.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditions {
    pub wind_speed_kmh: f64,
    pub wind_direction: String,
    pub temperature_celsius: f64,
    pub visibility_km: f64,
    pub rain: bool,
    pub description: String,
}

/// Verdict de sécurité pour le vol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyVerdict {
    pub safe: bool,
    pub reason: String,
    pub max_altitude: Option<f64>,
}

/// Événement de mission qui alimente l'orchestrateur.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MissionEvent {
    WaypointReached(GpsPosition),
    ObjectDetected(DetectedObject),
    BatteryLow(BatteryStatus),
    WeatherDegraded(WeatherConditions),
    ZoneCovered(f64),
    PeriodicTick,
    MissionStart(String),
    MissionEnd(String),
}

/// Statut courant de la mission en base de données.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionStatus {
    pub id: Uuid,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub waypoints_visited: u32,
    pub total_waypoints: u32,
    pub discoveries: u32,
    pub coverage_percent: f64,
    pub elapsed_seconds: u64,
}

/// Découverte enregistrée.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: Uuid,
    pub position: GpsPosition,
    pub description: String,
    pub photo_path: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Point de passage (waypoint) sur la carte.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapWaypoint {
    pub position: GpsPosition,
    pub label: String,
    pub photo_path: Option<String>,
    pub visited: bool,
}

/// Secteur de la grille d'exploration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sector {
    pub id: u32,
    pub center: GpsPosition,
    pub priority: u32,
    pub reason: String,
    pub explored: bool,
}
