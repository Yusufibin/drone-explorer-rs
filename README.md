# Drone Explorer RS

Station au sol expérimentale en Rust : agent de planification, cinq services JSON-RPC locaux, carte et journal SQLite. **La version actuelle sert aux essais en simulation uniquement. Elle ne doit pas piloter un drone réel.**

## État des intégrations

| Composant | Disponible | Limite |
| --- | --- | --- |
| Vol | `sim://mavlink` | Commandes et télémétrie MAVLink réelles non implémentées |
| Caméra et détection | `sim://camera`, `sim://yolo` | Capture et inférence réelles non implémentées |
| Météo | OpenWeatherMap ou `sim://openweather` | Le verdict simulé sert uniquement aux essais |
| Carte et missions | Carte en mémoire, SQLite | La géofence simulée est centrée sur le point initial codé dans le simulateur |
| Agent | LLM via OpenRouter ou décisions simulées | Les décisions simulées sont activées uniquement avec `simulation.enabled: true` |

Les cinq services écoutent sur `127.0.0.1` aux ports 9001 à 9005. Le protocole est JSON-RPC via `jsonrpsee` ; il ne fournit pas les méthodes de découverte et de transport d'un serveur MCP standard. Ne l'exposez pas sur un réseau non fiable : il n'y a pas d'authentification RPC.

## Démonstration locale

Prérequis : Rust stable, Cargo. Depuis la racine du dépôt, ouvrir six terminaux :

```bash
cargo run --release --bin flight-server -- --mavlink-url sim://mavlink
cargo run --release --bin vision-server -- --stream-url sim://camera --model-path sim://yolo
cargo run --release --bin weather-server -- --api-key sim://openweather
cargo run --release --bin map-server
cargo run --release --bin mission-server
cargo run --release --bin agent -- --config config/simulation.yaml --mission "Explorer la grille de simulation"
```

Les commandes ci-dessus utilisent les paramètres de simulation de `config/simulation.yaml`. Le fichier `config/config.yaml` montre les paramètres envisagés pour un futur système réel, mais le chargement en mode réel est volontairement refusé. Les ports, coordonnées et limites déclarés dans le fichier de l'agent doivent correspondre aux arguments des services démarrés séparément ; aucune distribution automatique de configuration n'est fournie.

`Ctrl+C` demande un RTL au simulateur. Les missions sont enregistrées dans `data/mission.db`. Les snapshots simulés sont des fichiers de métadonnées JSON, et non des images. Les répertoires `data/`, `snapshots/`, `reports/` sont ignorés par Git.

## Contrôles appliqués

- Au démarrage : refus du mode réel et validation des paramètres essentiels de la configuration.
- Sur le serveur de vol : refus des coordonnées non finies ou hors plage, du dépassement d'altitude, de rayon et de vitesse, et du décollage ou déplacement avec batterie sous le seuil.
- Dans l'agent : refus de poursuivre lorsque la batterie, la télémétrie ou le verdict météo manquent. Une erreur ou un arrêt demande un RTL et remonte l'échec si le RPC RTL échoue.
- Le LLM ne passe jamais silencieusement en simulation après une panne réseau ou une erreur HTTP.

Ces contrôles ne constituent pas un dispositif de sûreté aéronautique. Il reste à implémenter puis qualifier les commandes et accusés de réception MAVLink, la télémétrie fraîche et authentifiée, les délais et pertes de liaison, une géofence indépendante de l'agent, la capture et l'inférence réelles, la supervision humaine et les essais SITL et matériels. L'arrivée à un waypoint est encore synthétique dans l'agent de simulation.

## Développement

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Le CI exécute ces commandes. La licence est MIT.
