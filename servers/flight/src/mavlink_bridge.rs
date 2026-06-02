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
}

impl MavlinkBridge {
    /// Crée un nouveau pont de communication.
    pub fn new(connection_string: &str) -> Self {
        Self {
            connection_string: connection_string.to_string(),
            simulation: connection_string.starts_with("sim://"),
            state: Arc::new(Mutex::new(SimState::default())),
        }
    }

    /// Connecte le pont (lance le simulateur physique en tâche de fond).
    pub async fn connect(&self) -> Result<(), DroneError> {
        tracing::info!("Connexion MAVLink sur {}...", self.connection_string);

        if !self.simulation {
            mavlink::connect::<mavlink::ardupilotmega::MavMessage>(&self.connection_string)
                .map_err(|e| DroneError::Mavlink(format!("Connexion MAVLink impossible: {}", e)))?;
            tracing::info!("Connexion MAVLink réelle validée.");
            return Ok(());
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
                    if state.mode == FlightMode::Guided || state.mode == FlightMode::Auto {
                        let d_lat = target.lat - state.position.lat;
                        let d_lon = target.lon - state.position.lon;
                        let d_alt = target.alt - state.position.alt;

                        let dist = (d_lat.powi(2) + d_lon.powi(2)).sqrt();

                        if dist < 0.00005 && d_alt.abs() < 0.5 {
                            // Cible atteinte
                            state.position = target;
                            state.target_position = None;
                            state.speed = 0.0;
                            tracing::info!("Waypoint atteint: {}", target);
                        } else {
                            // Avancer vers la cible
                            state.speed = 5.0; // 5 m/s

                            // Calcul du cap (heading)
                            state.heading = (d_lon.atan2(d_lat).to_degrees() + 360.0) % 360.0;

                            // Pas de déplacement GPS simulé
                            let step = 0.00002; // environ 2 mètres
                            if dist > 0.0 {
                                state.position.lat += (d_lat / dist) * step;
                                state.position.lon += (d_lon / dist) * step;
                            }

                            // Pas d'altitude
                            if d_alt.abs() > 0.1 {
                                state.position.alt += d_alt.signum() * 0.5;
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
        let mut state = self.state.lock().await;
        self.ensure_simulation_control()?;
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
        let mut state = self.state.lock().await;
        self.ensure_simulation_control()?;
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
        let mut state = self.state.lock().await;
        self.ensure_simulation_control()?;
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
        max_alt: f64,
        min_battery: f32,
    ) -> Result<bool, DroneError> {
        self.ensure_simulation_control()?;
        let battery = self.get_battery().await?;

        if battery.percent < min_battery {
            tracing::error!("Refus GOTO: Batterie faible ({:.1}%)", battery.percent);
            self.rtl().await?;
            return Err(DroneError::BatteryCritical(battery.percent as f64));
        }

        if alt > max_alt {
            tracing::error!(
                "Refus GOTO: Altitude demandée ({:.1}m) supérieure au max légal ({:.1}m)",
                alt,
                max_alt
            );
            return Err(DroneError::GeofenceViolation(alt));
        }

        self.goto(lat, lon, alt).await
    }
}
