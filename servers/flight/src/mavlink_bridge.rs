use chrono::Utc;
use common::error::DroneError;
use common::types::{BatteryStatus, FlightMode, GpsPosition, Telemetry};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// État simulé du drone pour le développement et la simulation SITL.
#[derive(Debug, Clone)]
pub struct SimState {
    pub position: GpsPosition,
    pub target_position: Option<GpsPosition>,
    pub battery_percent: f32,
    pub battery_voltage: f32,
    pub armed: bool,
    pub mode: FlightMode,
    pub speed: f64,
    pub heading: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn simulator_requires_explicit_url() {
        let bridge = MavlinkBridge::new("sim://mavlink");

        bridge.connect().await.unwrap();
        bridge.takeoff(12.0).await.unwrap();

        let telemetry = bridge.get_telemetry().await.unwrap();
        assert_eq!(telemetry.mode, FlightMode::Guided);
        assert!(telemetry.armed);
    }

    #[tokio::test]
    async fn real_control_path_is_not_silently_simulated() {
        let bridge = MavlinkBridge::new("udpin:0.0.0.0:14550");

        let err = bridge.takeoff(12.0).await.unwrap_err();

        assert!(err.to_string().contains("Contrôle MAVLink réel non câblé"));
    }

    #[tokio::test]
    async fn rejects_invalid_takeoff_and_targets() {
        let bridge = MavlinkBridge::new("sim://mavlink").with_limits(30.0, 8.0, 100.0, 25.0).unwrap();
        bridge.connect().await.unwrap();
        assert!(bridge.takeoff(f64::NAN).await.is_err());
        assert!(bridge.takeoff(31.0).await.is_err());
        bridge.takeoff(10.0).await.unwrap();
        assert!(bridge.goto(0.0, 0.0, 10.0).await.is_err());
        assert!(bridge.goto(48.8566, 2.3522, -1.0).await.is_err());
        assert!(bridge.set_speed(f64::INFINITY).await.is_err());
        assert!(bridge.set_speed(9.0).await.is_err());
    }

    #[tokio::test]
    async fn refuses_flight_on_low_battery() {
        let bridge = MavlinkBridge::new("sim://mavlink");
        bridge.state.lock().await.battery_percent = 20.0;
        assert!(matches!(bridge.takeoff(10.0).await, Err(DroneError::BatteryCritical(_))));
    }
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            position: GpsPosition {
                lat: 48.8566,
                lon: 2.3522,
                alt: 0.0,
            },
            target_position: None,
            battery_percent: 100.0,
            battery_voltage: 16.8, // 4S pleine charge
            armed: false,
            mode: FlightMode::Manual,
            speed: 0.0,
            heading: 0.0,
        }
    }
}

/// Pont de communication MAVLink (ou simulation active).
#[derive(Clone)]
pub struct MavlinkBridge {
    pub connection_string: String,
    simulation: bool,
    state: Arc<Mutex<SimState>>,
    max_altitude: f64,
    max_speed: f64,
    geofence_radius: f64,
    min_battery: f32,
}

impl MavlinkBridge {
    /// Crée un nouveau pont de communication.
    pub fn new(connection_string: &str) -> Self {
        Self {
            connection_string: connection_string.to_string(),
            simulation: connection_string.starts_with("sim://"),
            state: Arc::new(Mutex::new(SimState::default())),
            max_altitude: 80.0,
            max_speed: 15.0,
            geofence_radius: 200.0,
            min_battery: 25.0,
        }
    }

    pub fn with_limits(mut self, max_altitude: f64, max_speed: f64, geofence_radius: f64, min_battery: f32) -> Result<Self, DroneError> {
        if !max_altitude.is_finite() || max_altitude <= 0.0 || !max_speed.is_finite() || max_speed <= 0.0
            || !geofence_radius.is_finite() || geofence_radius <= 0.0 || !min_battery.is_finite()
            || !(0.0..=100.0).contains(&min_battery) {
            return Err(DroneError::Config("Limites de vol invalides".into()));
        }
        self.max_altitude = max_altitude;
        self.max_speed = max_speed;
        self.geofence_radius = geofence_radius;
        self.min_battery = min_battery;
        Ok(self)
    }

    /// Connecte le pont (lance le simulateur physique en tâche de fond).
    pub async fn connect(&self) -> Result<(), DroneError> {
        tracing::info!("Connexion MAVLink sur {}...", self.connection_string);

        if !self.simulation {
            return Err(DroneError::Mavlink("Vol réel désactivé : commandes et télémétrie MAVLink non implémentées. Utiliser sim://mavlink pour les essais.".into()));
        }

        tokio::time::sleep(Duration::from_millis(500)).await;

        let state_clone = self.state.clone();

        // Lancer la boucle de simulation physique en arrière-plan
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                let mut state = state_clone.lock().await;

                // 1. Simulation de la batterie qui se décharge doucement
                if state.armed {
                    state.battery_percent -= 0.05; // Décharge
                    state.battery_voltage -= 0.002;
                    if state.battery_percent < 0.0 {
                        state.battery_percent = 0.0;
                    }
                }

                // 2. Déplacement physique si en mode GUIDED/AUTO et vers une cible
                if let Some(target) = state.target_position {
                    if matches!(state.mode, FlightMode::Guided | FlightMode::Auto | FlightMode::Rtl | FlightMode::Land) {
                        let d_lat = target.lat - state.position.lat;
                        let d_lon = target.lon - state.position.lon;
                        let d_alt = target.alt - state.position.alt;

                        let dist = (d_lat.powi(2) + d_lon.powi(2)).sqrt();

                        if dist < 0.00005 && d_alt.abs() < 0.5 {
                            // Cible atteinte
                            state.position = target;
                            state.target_position = None;
                            state.speed = 0.0;
                            if matches!(state.mode, FlightMode::Rtl | FlightMode::Land) {
                                state.armed = false;
                            }
                            tracing::info!("Waypoint atteint: {}", target);
                        } else {
                            // Avancer vers la cible
                            state.speed = 5.0; // 5 m/s

                            // Calcul du cap (heading)
                            state.heading = (d_lon.atan2(d_lat).to_degrees() + 360.0) % 360.0;

                            // Pas de déplacement GPS simulé
                            let step = 0.00002_f64.min(dist);
                            if dist > 0.0 {
                                state.position.lat += (d_lat / dist) * step;
                                state.position.lon += (d_lon / dist) * step;
                            }

                            // Pas d'altitude
                            if d_alt.abs() > 0.1 {
                                state.position.alt += d_alt.signum() * 0.5_f64.min(d_alt.abs());
                            }
                        }
                    }
                }
            }
        });

        tracing::warn!("MAVLink en mode simulation explicite.");
        Ok(())
    }

    fn ensure_simulation_control(&self) -> Result<(), DroneError> {
        if self.simulation {
            Ok(())
        } else {
            Err(DroneError::Mavlink(
                "Contrôle MAVLink réel non câblé dans ce serveur Rust. Le lien est validé, mais les commandes exigent une implémentation MAVLink/MAVSDK complète conforme au plan.".into(),
            ))
        }
    }

    /// Arme les moteurs et décolle à l'altitude spécifiée.
    pub async fn takeoff(&self, altitude: f64) -> Result<bool, DroneError> {
        self.ensure_simulation_control()?;
        self.validate_altitude(altitude)?;
        let mut state = self.state.lock().await;
        if state.battery_percent < self.min_battery {
            return Err(DroneError::BatteryCritical(state.battery_percent as f64));
        }
        tracing::info!("Commande de décollage reçue à {:.1}m", altitude);

        state.armed = true;
        state.mode = FlightMode::Guided;
        let current_pos = state.position;
        state.target_position = Some(GpsPosition {
            lat: current_pos.lat,
            lon: current_pos.lon,
            alt: altitude,
        });

        Ok(true)
    }

    /// Déplace le drone vers des coordonnées GPS données.
    pub async fn goto(&self, lat: f64, lon: f64, alt: f64) -> Result<bool, DroneError> {
        self.ensure_simulation_control()?;
        self.validate_target(lat, lon, alt)?;
        let mut state = self.state.lock().await;
        if state.battery_percent < self.min_battery {
            return Err(DroneError::BatteryCritical(state.battery_percent as f64));
        }
        tracing::info!(
            "Commande GOTO reçue vers ({:.6}, {:.6}, {:.1}m)",
            lat,
            lon,
            alt
        );

        if !state.armed {
            return Err(DroneError::Mavlink(
                "Le drone doit être armé avant de naviguer".into(),
            ));
        }

        state.mode = FlightMode::Guided;
        state.target_position = Some(GpsPosition { lat, lon, alt });
        Ok(true)
    }

    /// Atterrit sur place immédiatement.
    pub async fn land(&self) -> Result<bool, DroneError> {
        let mut state = self.state.lock().await;
        self.ensure_simulation_control()?;
        tracing::warn!("Commande d'atterrissage immédiat (LAND) reçue");

        state.mode = FlightMode::Land;
        let current_pos = state.position;
        state.target_position = Some(GpsPosition {
            lat: current_pos.lat,
            lon: current_pos.lon,
            alt: 0.0,
        });

        Ok(true)
    }

    /// Retourne au point de départ (RTL - Return-To-Launch).
    pub async fn rtl(&self) -> Result<bool, DroneError> {
        let mut state = self.state.lock().await;
        self.ensure_simulation_control()?;
        tracing::warn!("Commande de retour au point de départ (RTL) reçue");

        state.mode = FlightMode::Rtl;
        // Simuler le retour aux coordonnées initiales (Paris)
        state.target_position = Some(GpsPosition {
            lat: 48.8566,
            lon: 2.3522,
            alt: 0.0,
        });

        Ok(true)
    }

    /// Récupère la télémétrie courante du drone.
    pub async fn get_telemetry(&self) -> Result<Telemetry, DroneError> {
        self.ensure_simulation_control()?;
        let state = self.state.lock().await;
        Ok(Telemetry {
            position: state.position,
            heading: state.heading,
            speed: state.speed,
            mode: state.mode,
            armed: state.armed,
            timestamp: Utc::now(),
        })
    }

    /// Récupère l'état de la batterie.
    pub async fn get_battery(&self) -> Result<BatteryStatus, DroneError> {
        self.ensure_simulation_control()?;
        let state = self.state.lock().await;
        let remaining_minutes = if state.armed {
            Some(state.battery_percent * 0.15) // ~15 minutes max
        } else {
            Some(30.0)
        };

        Ok(BatteryStatus {
            percent: state.battery_percent,
            voltage: state.battery_voltage,
            remaining_minutes,
        })
    }

    /// Vol stationnaire (Loiter) pendant une durée.
    pub async fn loiter(&self, duration_secs: u64) -> Result<bool, DroneError> {
        let mut state = self.state.lock().await;
        self.ensure_simulation_control()?;
        tracing::info!("Commande LOITER reçue pour {} secondes", duration_secs);

        state.mode = FlightMode::Loiter;
        state.target_position = None;
        state.speed = 0.0;

        Ok(true)
    }

    /// Règle la vitesse de déplacement en m/s.
    pub async fn set_speed(&self, speed: f64) -> Result<bool, DroneError> {
        self.ensure_simulation_control()?;
        if !speed.is_finite() || speed <= 0.0 || speed > self.max_speed {
            return Err(DroneError::Config("Vitesse hors limites".into()));
        }
        let mut state = self.state.lock().await;
        tracing::info!("Vitesse fixée à {:.1} m/s", speed);

        state.speed = speed;
        Ok(true)
    }

    /// Effectue un goto sécurisé avec vérification batterie et altitude.
    pub async fn safe_goto(
        &self,
        lat: f64,
        lon: f64,
        alt: f64,
        _max_alt: f64,
        _min_battery: f32,
    ) -> Result<bool, DroneError> {
        self.goto(lat, lon, alt).await
    }

    fn validate_altitude(&self, alt: f64) -> Result<(), DroneError> {
        if !alt.is_finite() || alt <= 0.0 || alt > self.max_altitude {
            return Err(DroneError::GeofenceViolation(alt));
        }
        Ok(())
    }

    fn validate_target(&self, lat: f64, lon: f64, alt: f64) -> Result<(), DroneError> {
        self.validate_altitude(alt)?;
        if !lat.is_finite() || !lon.is_finite() || !(-90.0..=90.0).contains(&lat)
            || !(-180.0..=180.0).contains(&lon) {
            return Err(DroneError::Config("Coordonnées GPS invalides".into()));
        }
        let home = SimState::default().position;
        let lat_m = (lat - home.lat).to_radians() * 6_371_000.0;
        let lon_m = (lon - home.lon).to_radians() * home.lat.to_radians().cos() * 6_371_000.0;
        let distance = lat_m.hypot(lon_m);
        if distance > self.geofence_radius {
            return Err(DroneError::GeofenceViolation(distance));
        }
        Ok(())
    }
}
