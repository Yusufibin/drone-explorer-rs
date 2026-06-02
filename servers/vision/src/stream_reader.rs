use chrono::{DateTime, Utc};
use common::error::DroneError;

/// Représente une image capturée du flux vidéo.
#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn real_stream_without_backend_fails_explicitly() {
        let mut reader = StreamReader::new("udp://0.0.0.0:5600");

        let err = reader.connect().await.unwrap_err();

        assert!(err.to_string().contains("Flux vidéo réel indisponible"));
    }

    #[tokio::test]
    async fn simulation_stream_is_explicit() {
        let mut reader = StreamReader::new("sim://camera");

        reader.connect().await.unwrap();
        let frame = reader.capture_frame().await.unwrap();

        assert_eq!(frame.width, 1920);
        assert!(reader.simulation);
    }
}

/// Lecteur de flux vidéo (simulé ou réel).
#[derive(Debug, Clone)]
pub struct StreamReader {
    pub url: String,
    pub connected: bool,
    pub simulation: bool,
}

impl StreamReader {
    /// Crée un nouveau lecteur de flux.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            connected: false,
            simulation: url.starts_with("sim://"),
        }
    }

    /// Tente de se connecter au flux de la caméra.
    pub async fn connect(&mut self) -> Result<(), DroneError> {
        tracing::info!(
            "Tentative de connexion au flux vidéo RTSP/UDP: {}...",
            self.url
        );
        if self.simulation {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            self.connected = true;
            tracing::warn!("Vision en mode simulation explicite.");
            return Ok(());
        }

        #[cfg(feature = "opencv-backend")]
        {
            use opencv::prelude::VideoCaptureTraitConst;
            use opencv::videoio::{CAP_ANY, VideoCapture};

            let capture = VideoCapture::from_file(&self.url, CAP_ANY)
                .map_err(|e| DroneError::Vision(format!("Connexion OpenCV impossible: {}", e)))?;
            if capture
                .is_opened()
                .map_err(|e| DroneError::Vision(format!("Vérification flux impossible: {}", e)))?
            {
                self.connected = true;
                tracing::info!("Flux vidéo connecté avec succès.");
                return Ok(());
            }
        }

        Err(DroneError::Vision(
            "Flux vidéo réel indisponible. Compile avec la feature `opencv-backend` et fournis un flux RTSP/UDP valide, ou utilise `sim://camera` explicitement.".into(),
        ))
    }

    /// Capture l'image courante.
    pub async fn capture_frame(&self) -> Result<Frame, DroneError> {
        if !self.connected {
            return Err(DroneError::Vision("Flux vidéo non connecté".into()));
        }

        if !self.simulation {
            return Err(DroneError::Vision(
                "Capture vidéo réelle non active dans ce binaire. Active `opencv-backend` pour lire le flux caméra.".into(),
            ));
        }

        Ok(Frame {
            width: 1920,
            height: 1080,
            data: vec![0; 100], // Mock minimal data
            timestamp: Utc::now(),
        })
    }

    /// Sauvegarde l'image courante sous forme de snapshot.
    pub async fn save_snapshot(&self, dir: &str) -> Result<String, DroneError> {
        let frame = self.capture_frame().await?;
        std::fs::create_dir_all(dir).map_err(|e| {
            DroneError::Vision(format!("Impossible de créer le dossier snapshot: {}", e))
        })?;

        let filepath = format!(
            "{}/snapshot_{}.json",
            dir,
            frame.timestamp.timestamp_millis()
        );
        let meta = serde_json::json!({
            "width": frame.width,
            "height": frame.height,
            "timestamp": frame.timestamp,
            "stream_url": self.url,
            "simulation": self.simulation
        });

        std::fs::write(&filepath, meta.to_string())
            .map_err(|e| DroneError::Vision(format!("Impossible d'écrire le snapshot: {}", e)))?;

        tracing::info!("Snapshot sauvegardé sous {}", filepath);
        Ok(filepath)
    }
}
