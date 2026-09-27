use common::error::DroneError;
use common::types::{SafetyVerdict, WeatherConditions};
use serde::{Deserialize, Serialize};

/// Client pour l'API OpenWeatherMap.
#[derive(Debug, Clone)]
pub struct OpenWeatherClient {
    pub api_key: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct Wind {
    speed: f64, // m/s
    deg: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Main {
    temp: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct Weather {
    description: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Rain {
    #[serde(rename = "1h")]
    one_h: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenWeatherResponse {
    wind: Wind,
    main: Main,
    visibility: Option<f64>, // mètres
    weather: Vec<Weather>,
    rain: Option<Rain>,
}

impl OpenWeatherClient {
    /// Crée un nouveau client OpenWeatherMap.
    pub fn new(api_key: &str, lat: f64, lon: f64) -> Self {
        Self {
            api_key: api_key.to_string(),
            lat,
            lon,
        }
    }

    /// Récupère les conditions météorologiques en temps réel.
    pub async fn get_conditions(&self) -> Result<WeatherConditions, DroneError> {
        if self.api_key == "sim://openweather" {
            tracing::warn!("Météo en mode simulation explicite.");
            return Ok(self.get_mock_conditions());
        }
        if self.api_key.is_empty() || self.api_key == "${OPENWEATHER_API_KEY}" {
            return Err(DroneError::Weather(
                "Clé API OpenWeatherMap manquante. Définis OPENWEATHER_API_KEY ou utilise `sim://openweather` explicitement en développement.".into(),
            ));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build().map_err(|e| DroneError::Weather(e.to_string()))?;
        let lat = self.lat.to_string();
        let lon = self.lon.to_string();
        match client.get("https://api.openweathermap.org/data/2.5/weather")
            .query(&[("lat", lat.as_str()), ("lon", lon.as_str()), ("appid", self.api_key.as_str()), ("units", "metric")])
            .send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<OpenWeatherResponse>().await {
                        Ok(data) => {
                            let wind_speed_kmh = data.wind.speed * 3.6; // convert m/s to km/h
                            let wind_direction =
                                Self::wind_deg_to_direction(data.wind.deg.unwrap_or(0.0));
                            let visibility_km = (data.visibility.unwrap_or(10000.0)) / 1000.0;
                            let rain = data.rain.and_then(|r| r.one_h).unwrap_or(0.0) > 0.1;
                            let description = data
                                .weather
                                .first()
                                .map(|w| w.description.clone())
                                .unwrap_or_else(|| "Nuageux".to_string());

                            Ok(WeatherConditions {
                                wind_speed_kmh,
                                wind_direction,
                                temperature_celsius: data.main.temp,
                                visibility_km,
                                rain,
                                description,
                            })
                        }
                        Err(e) => Err(DroneError::Weather(format!(
                            "Erreur de désérialisation OpenWeatherMap: {}",
                            e
                        ))),
                    }
                } else {
                    Err(DroneError::Weather(format!(
                        "Réponse HTTP OpenWeatherMap en erreur: {}",
                        resp.status()
                    )))
                }
            }
            Err(e) => Err(DroneError::Weather(format!(
                "Erreur réseau OpenWeatherMap: {}",
                e.status().map(|s| s.to_string()).unwrap_or_else(|| "connexion ou délai dépassé".into())
            ))),
        }
    }

    /// Vérifie si les conditions permettent de voler en sécurité.
    pub async fn is_safe_to_fly(&self) -> Result<SafetyVerdict, DroneError> {
        let cond = self.get_conditions().await?;

        let mut reasons = Vec::new();
        let mut safe = true;

        // Vent max acceptable = 45 km/h
        if cond.wind_speed_kmh > 45.0 {
            safe = false;
            reasons.push(format!(
                "Vent trop fort ({:.1} km/h > 45 km/h)",
                cond.wind_speed_kmh
            ));
        }

        // Visibilité min = 1 km
        if cond.visibility_km < 1.0 {
            safe = false;
            reasons.push(format!(
                "Visibilité insuffisante ({:.1} km < 1 km)",
                cond.visibility_km
            ));
        }

        // Pas de pluie
        if cond.rain {
            safe = false;
            reasons.push("Pluie en cours".to_string());
        }

        let reason = if safe {
            "Conditions de vol idéales.".to_string()
        } else {
            reasons.join(", ")
        };

        let max_altitude = if safe {
            Some(80.0) // 80m max
        } else {
            None
        };

        Ok(SafetyVerdict {
            safe,
            reason,
            max_altitude,
        })
    }

    /// Retourne des données météo simulées de qualité.
    fn get_mock_conditions(&self) -> WeatherConditions {
        WeatherConditions {
            wind_speed_kmh: 15.4,
            wind_direction: "NE".to_string(),
            temperature_celsius: 19.5,
            visibility_km: 10.0,
            rain: false,
            description: "Ciel partiellement nuageux".to_string(),
        }
    }

    /// Convertit les degrés de vent en direction cardinale.
    fn wind_deg_to_direction(deg: f64) -> String {
        let directions = [
            "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
            "NW", "NNW",
        ];
        let idx = (((deg + 11.25) % 360.0) / 22.5) as usize;
        directions[idx % 16].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_api_key_is_not_silently_mocked() {
        let client = OpenWeatherClient::new("", 48.8566, 2.3522);

        let err = client.get_conditions().await.unwrap_err();

        assert!(err.to_string().contains("Clé API OpenWeatherMap manquante"));
    }

    #[tokio::test]
    async fn simulation_requires_explicit_key() {
        let client = OpenWeatherClient::new("sim://openweather", 48.8566, 2.3522);

        let conditions = client.get_conditions().await.unwrap();

        assert!(!conditions.rain);
        assert!(conditions.wind_speed_kmh < 45.0);
    }
}
