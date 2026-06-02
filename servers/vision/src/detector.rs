use common::types::{BoundingBox, DetectedObject, GpsPosition, SafetyVerdict, TerrainType};
use rand::Rng;

use crate::stream_reader::Frame;
use common::error::DroneError;

/// Détecteur d'objets (YOLO simulé ou réel).
pub struct ObjectDetector {
    pub model_path: String,
    pub min_confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn missing_yolo_model_is_not_silently_mocked() {
        let detector = ObjectDetector::new("models/missing.pt", 0.5);
        let frame = Frame {
            width: 640,
            height: 480,
            data: vec![],
            timestamp: Utc::now(),
        };

        let err = detector.detect(&frame).await.unwrap_err();

        assert!(err.to_string().contains("Modèle YOLO introuvable"));
    }

    #[tokio::test]
    async fn simulation_detector_is_explicit() {
        let detector = ObjectDetector::new("sim://yolo", 0.5);
        let frame = Frame {
            width: 640,
            height: 480,
            data: vec![],
            timestamp: Utc::now(),
        };

        let detections = detector.detect(&frame).await.unwrap();

        assert!(detections.len() <= 3);
    }
}

impl ObjectDetector {
    /// Initialise le détecteur.
    pub fn new(model_path: &str, min_confidence: f32) -> Self {
        Self {
            model_path: model_path.to_string(),
            min_confidence,
        }
    }

    /// Détecte des objets dans l'image spécifiée.
    pub async fn detect(&self, _frame: &Frame) -> Result<Vec<DetectedObject>, DroneError> {
        if !self.model_path.starts_with("sim://") {
            if !std::path::Path::new(&self.model_path).exists() {
                return Err(DroneError::Vision(format!(
                    "Modèle YOLO introuvable: '{}'. Fournis un modèle réel ou utilise `sim://yolo` explicitement.",
                    self.model_path
                )));
            }

            return Err(DroneError::Vision(
                "Inférence YOLO réelle non câblée dans ce binaire Rust. Le plan exige YOLO; ajoute un backend ONNX/OpenCV DNN ou utilise `sim://yolo` pour le développement.".into(),
            ));
        }

        let mut rng = rand::thread_rng();
        let count = rng.gen_range(0..=3); // 0 à 3 objets
        let mut objects = Vec::new();

        let classes = [
            "Personne",
            "Voiture",
            "Bâtiment",
            "Arbre",
            "Obstacle",
            "Panneau Solaire",
        ];

        for _ in 0..count {
            let class_idx = rng.gen_range(0..classes.len());
            let confidence = rng.gen_range(0.45..1.0f32);

            if confidence >= self.min_confidence {
                objects.push(DetectedObject {
                    class_name: classes[class_idx].to_string(),
                    confidence,
                    bbox: BoundingBox {
                        x: rng.gen_range(100.0..800.0),
                        y: rng.gen_range(100.0..600.0),
                        width: rng.gen_range(50.0..200.0),
                        height: rng.gen_range(50.0..200.0),
                    },
                    // GPS approximatif basé sur la position courante (ajouté par le serveur si besoin)
                    position: Some(GpsPosition {
                        lat: 48.8566 + rng.gen_range(-0.001..0.001),
                        lon: 2.3522 + rng.gen_range(-0.001..0.001),
                        alt: 20.0,
                    }),
                });
            }
        }

        Ok(objects)
    }

    /// Classifie le type de terrain sous le drone.
    pub async fn classify_terrain(&self, _frame: &Frame) -> Result<TerrainType, DroneError> {
        if !self.model_path.starts_with("sim://") {
            return Err(DroneError::Vision(
                "Classification terrain réelle non disponible sans backend vision. Utilise `sim://yolo` uniquement en simulation.".into(),
            ));
        }

        let mut rng = rand::thread_rng();
        let roll = rng.gen_range(0..100);

        let terrain = match roll {
            0..=30 => TerrainType::Vegetation,
            31..=60 => TerrainType::Urban,
            61..=80 => TerrainType::Road,
            81..=90 => TerrainType::Water,
            _ => TerrainType::Forest,
        };

        Ok(terrain)
    }

    /// Vérifie si la zone sous-jacente est propice à un atterrissage sécurisé.
    pub async fn check_landing_zone(&self, _frame: &Frame) -> Result<SafetyVerdict, DroneError> {
        if !self.model_path.starts_with("sim://") {
            return Err(DroneError::Vision(
                "Analyse réelle de zone d'atterrissage non disponible sans backend vision. Utilise `sim://yolo` uniquement en simulation.".into(),
            ));
        }

        let mut rng = rand::thread_rng();
        let safe = rng.gen_bool(0.7); // 70% de chance d'être safe

        if safe {
            Ok(SafetyVerdict {
                safe: true,
                reason: "Zone d'atterrissage plane et dégagée.".to_string(),
                max_altitude: None,
            })
        } else {
            Ok(SafetyVerdict {
                safe: false,
                reason: "Obstacle ou végétation trop dense détecté au sol.".to_string(),
                max_altitude: None,
            })
        }
    }
}
