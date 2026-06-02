use common::types::{GpsPosition, Sector};

/// Gère les algorithmes de choix de chemin et d'exploration frontier.
pub struct FrontierExplorer;

impl FrontierExplorer {
    /// Trouve le prochain meilleur secteur inexploré.
    ///
    /// Sélectionne en priorité les secteurs non explorés, triés par priorité décroissante,
    /// puis par distance croissante depuis la position courante du drone.
    pub fn get_unexplored_sector(
        &self,
        sectors: &[Sector],
        current_pos: &GpsPosition,
    ) -> Option<Sector> {
        let mut unexplored: Vec<Sector> = sectors.iter().filter(|s| !s.explored).cloned().collect();

        if unexplored.is_empty() {
            return None;
        }

        // Tri multicritères :
        // 1. Priorité la plus haute d'abord (descendant)
        // 2. Distance la plus proche d'abord (ascendant)
        unexplored.sort_by(|a, b| {
            let prio_cmp = b.priority.cmp(&a.priority);
            if prio_cmp == std::cmp::Ordering::Equal {
                let dist_a = Self::calculate_distance(&a.center, current_pos);
                let dist_b = Self::calculate_distance(&b.center, current_pos);
                dist_a
                    .partial_cmp(&dist_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                prio_cmp
            }
        });

        unexplored.first().cloned()
    }

    /// Calcule la distance de Haversine entre deux coordonnées GPS.
    pub fn calculate_distance(a: &GpsPosition, b: &GpsPosition) -> f64 {
        let r = 6371000.0; // Rayon de la Terre en mètres
        let d_lat = (b.lat - a.lat).to_radians();
        let d_lon = (b.lon - a.lon).to_radians();

        let lat1 = a.lat.to_radians();
        let lat2 = b.lat.to_radians();

        let sin_lat = (d_lat / 2.0).sin();
        let sin_lon = (d_lon / 2.0).sin();

        let term = sin_lat * sin_lat + sin_lon * sin_lon * lat1.cos() * lat2.cos();
        let c = 2.0 * term.sqrt().asin();

        r * c
    }

    /// Propose un ordre optimal d'exploration (Heuristique du plus proche voisin).
    pub fn suggest_exploration_order(
        &self,
        sectors: &[Sector],
        start: &GpsPosition,
    ) -> Vec<Sector> {
        let mut unexplored: Vec<Sector> = sectors.iter().filter(|s| !s.explored).cloned().collect();

        let mut ordered = Vec::new();
        let mut current_pos = *start;

        while !unexplored.is_empty() {
            let mut nearest_idx = 0;
            let mut min_dist = f64::MAX;

            for (idx, sector) in unexplored.iter().enumerate() {
                let dist = Self::calculate_distance(&sector.center, &current_pos);
                // On pondère la distance par la priorité du secteur
                let weight = dist / (sector.priority as f64).max(1.0);
                if weight < min_dist {
                    min_dist = weight;
                    nearest_idx = idx;
                }
            }

            let next_sector = unexplored.remove(nearest_idx);
            current_pos = next_sector.center;
            ordered.push(next_sector);
        }

        ordered
    }
}
