use chrono::{DateTime, Utc};
use common::error::DroneError;
use common::types::{Finding, GpsPosition, MissionStatus};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Base de données SQLite pour stocker les états et logs de mission.
#[derive(Clone)]
pub struct MissionDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl MissionDatabase {
    /// Ouvre ou crée la base de données SQLite.
    pub fn new(db_path: &str) -> Result<Self, DroneError> {
        // S'assurer que le dossier parent existe
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DroneError::Database(format!(
                    "Impossible de créer le dossier de base de données: {}",
                    e
                ))
            })?;
        }

        let conn = Connection::open(db_path).map_err(|e| {
            DroneError::Database(format!("Erreur d'ouverture SQLite '{}': {}", db_path, e))
        })?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_tables()?;
        Ok(db)
    }

    /// Initialise la structure des tables si elles n'existent pas.
    pub fn init_tables(&self) -> Result<(), DroneError> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS missions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                summary TEXT
            );",
            [],
        )
        .map_err(|e| DroneError::Database(format!("Erreur création table missions: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS findings (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL,
                lat REAL NOT NULL,
                lon REAL NOT NULL,
                alt REAL,
                description TEXT NOT NULL,
                photo_path TEXT,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (mission_id) REFERENCES missions(id)
            );",
            [],
        )
        .map_err(|e| DroneError::Database(format!("Erreur création table findings: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mission_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                data TEXT,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (mission_id) REFERENCES missions(id)
            );",
            [],
        )
        .map_err(|e| DroneError::Database(format!("Erreur création table events: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS waypoints_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mission_id TEXT NOT NULL,
                lat REAL NOT NULL,
                lon REAL NOT NULL,
                alt REAL,
                reached_at TEXT NOT NULL,
                FOREIGN KEY (mission_id) REFERENCES missions(id)
            );",
            [],
        )
        .map_err(|e| DroneError::Database(format!("Erreur création table waypoints_log: {}", e)))?;

        Ok(())
    }

    /// Crée une nouvelle session de mission.
    pub fn create_mission(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> Result<String, DroneError> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let started_at = Utc::now().to_rfc3339();

        // Marquer les missions actives précédentes comme terminées
        let _ = conn.execute(
            "UPDATE missions SET status = 'completed', ended_at = ? WHERE status = 'active'",
            [started_at.clone()],
        );

        conn.execute(
            "INSERT INTO missions (id, name, description, started_at, status) VALUES (?, ?, ?, ?, 'active')",
            params![id, name, description, started_at],
        ).map_err(|e| DroneError::Database(format!("Impossible de créer la mission: {}", e)))?;

        Ok(id)
    }

    /// Termine la mission en cours.
    pub fn end_mission(&self, mission_id: &str, summary: &str) -> Result<(), DroneError> {
        let conn = self.conn.lock().unwrap();
        let ended_at = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE missions SET status = 'completed', ended_at = ?, summary = ? WHERE id = ?",
            params![ended_at, summary, mission_id],
        )
        .map_err(|e| DroneError::Database(format!("Impossible de terminer la mission: {}", e)))?;

        Ok(())
    }

    /// Enregistre une découverte.
    pub fn log_finding(&self, mission_id: &str, finding: &Finding) -> Result<(), DroneError> {
        let conn = self.conn.lock().unwrap();
        let id = finding.id.to_string();
        let timestamp = finding.timestamp.to_rfc3339();

        conn.execute(
            "INSERT INTO findings (id, mission_id, lat, lon, alt, description, photo_path, timestamp) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id,
                mission_id,
                finding.position.lat,
                finding.position.lon,
                finding.position.alt,
                finding.description,
                finding.photo_path,
                timestamp
            ],
        ).map_err(|e| DroneError::Database(format!("Impossible de loguer la découverte: {}", e)))?;

        Ok(())
    }

    /// Logue un événement général de mission.
    pub fn log_event(
        &self,
        mission_id: &str,
        event_type: &str,
        data: &serde_json::Value,
    ) -> Result<(), DroneError> {
        let conn = self.conn.lock().unwrap();
        let timestamp = Utc::now().to_rfc3339();
        let data_str = data.to_string();

        conn.execute(
            "INSERT INTO events (mission_id, event_type, data, timestamp) VALUES (?, ?, ?, ?)",
            params![mission_id, event_type, data_str, timestamp],
        )
        .map_err(|e| DroneError::Database(format!("Impossible de loguer l'événement: {}", e)))?;

        Ok(())
    }

    /// Logue un waypoint GPS visité.
    pub fn log_waypoint(
        &self,
        mission_id: &str,
        lat: f64,
        lon: f64,
        alt: f64,
    ) -> Result<(), DroneError> {
        let conn = self.conn.lock().unwrap();
        let reached_at = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO waypoints_log (mission_id, lat, lon, alt, reached_at) VALUES (?, ?, ?, ?, ?)",
            params![mission_id, lat, lon, alt, reached_at],
        ).map_err(|e| DroneError::Database(format!("Impossible de loguer le waypoint: {}", e)))?;

        Ok(())
    }

    /// Récupère le statut résumé d'une mission.
    pub fn get_mission_status(&self, mission_id: &str) -> Result<MissionStatus, DroneError> {
        let conn = self.conn.lock().unwrap();

        // 1. Lire les infos de base
        let mut stmt = conn
            .prepare("SELECT id, name, started_at FROM missions WHERE id = ?")
            .map_err(|e| DroneError::Database(e.to_string()))?;

        let (id_str, name, started_str): (String, String, String) = stmt
            .query_row([mission_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| {
                DroneError::Database(format!("Mission introuvable {}: {}", mission_id, e))
            })?;

        let id = Uuid::parse_str(&id_str).unwrap();
        let started_at = DateTime::parse_from_rfc3339(&started_str)
            .unwrap()
            .with_timezone(&Utc);

        // 2. Compter les waypoints logués
        let waypoints_visited: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM waypoints_log WHERE mission_id = ?",
                [mission_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // 3. Compter les découvertes
        let discoveries: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM findings WHERE mission_id = ?",
                [mission_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let elapsed_seconds = (Utc::now() - started_at).num_seconds() as u64;

        Ok(MissionStatus {
            id,
            name,
            started_at,
            waypoints_visited,
            total_waypoints: waypoints_visited + 5, // Estimation
            discoveries,
            coverage_percent: 0.0, // Rempli par l'orchestrateur avec map-mcp
            elapsed_seconds,
        })
    }

    /// Récupère toutes les découvertes pour une mission.
    pub fn get_findings(&self, mission_id: &str) -> Result<Vec<Finding>, DroneError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, lat, lon, alt, description, photo_path, timestamp FROM findings WHERE mission_id = ?")
            .map_err(|e| DroneError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([mission_id], |row| {
                let id_str: String = row.get(0)?;
                let id = Uuid::parse_str(&id_str).unwrap();
                let lat: f64 = row.get(1)?;
                let lon: f64 = row.get(2)?;
                let alt: f64 = row.get(3)?;
                let description: String = row.get(4)?;
                let photo_path: Option<String> = row.get(5)?;
                let ts_str: String = row.get(6)?;
                let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                    .unwrap()
                    .with_timezone(&Utc);

                Ok(Finding {
                    id,
                    position: GpsPosition { lat, lon, alt },
                    description,
                    photo_path,
                    timestamp,
                })
            })
            .map_err(|e| DroneError::Database(e.to_string()))?;

        let mut findings = Vec::new();
        for r in rows {
            if let Ok(finding) = r {
                findings.push(finding);
            }
        }

        Ok(findings)
    }

    /// Retourne la mission active (s'il y en a une).
    pub fn get_active_mission(&self) -> Result<Option<String>, DroneError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id FROM missions WHERE status = 'active' LIMIT 1")
            .map_err(|e| DroneError::Database(e.to_string()))?;

        let mut rows = stmt
            .query([])
            .map_err(|e| DroneError::Database(e.to_string()))?;
        if let Some(row) = rows
            .next()
            .map_err(|e| DroneError::Database(e.to_string()))?
        {
            let id: String = row.get(0).unwrap();
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }
}
