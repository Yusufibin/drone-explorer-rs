# Drone Explorer RS

![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust)
![MCP](https://img.shields.io/badge/MCP-JSON--RPC-2563eb)
![Async](https://img.shields.io/badge/Runtime-Tokio-0f766e)
![Database](https://img.shields.io/badge/Storage-SQLite-003b57?logo=sqlite)
![License](https://img.shields.io/badge/License-MIT-green)

Station au sol autonome pour drone, ecrite en Rust, pilotee par un agent LLM via des serveurs MCP. Le drone reste un capteur/executant MAVLink, tandis que la decision, la planification, la cartographie, la vision et les journaux de mission tournent au sol.

## Points forts

- **Agent ReAct autonome** : orchestration de mission, appels MCP, suivi d'evenements et decisions de retour securise.
- **Architecture microservices MCP** : chaque capacite du systeme expose une API JSON-RPC dediee.
- **Pont MAVLink** : commandes `takeoff`, `goto`, `land`, `rtl` et telemetrie, avec simulation physique integree.
- **Exploration frontier** : decoupage de zone en secteurs, selection du prochain secteur inexplore et export cartographique.
- **Vision et snapshots** : lecture de flux UDP/RTSP, detection d'objets et captures persistantes.
- **Memoire de mission SQLite** : missions, evenements, waypoints et decouvertes geolocalisees.
- **Failsafes sol** : batterie critique, meteo defavorable, altitude maximale et retour prioritaire.

## Architecture

```text
drone-explorer-rs/
├── agent/                  # Orchestrateur LLM, boucle ReAct et clients MCP
├── common/                 # Types, erreurs, configuration et contrats partages
├── servers/
│   ├── flight/             # MCP MAVLink: vol, RTL, telemetrie
│   ├── vision/             # MCP vision: flux video, detection, snapshots
│   ├── weather/            # MCP meteo: conditions de vol
│   ├── map/                # MCP carte: secteurs, frontier, GeoJSON
│   └── mission/            # MCP mission: SQLite, logs, findings
├── config/                 # Configuration YAML
├── onboard/                # Scripts Raspberry Pi / drone
└── data/                   # Donnees runtime locales ignorees par Git
```

## Composants

| Binaire | Port | Role |
| --- | ---: | --- |
| `agent` | - | Agent principal et boucle de decision |
| `flight-server` | `9001` | Interface MAVLink et commandes de vol |
| `vision-server` | `9002` | Analyse video, detection et snapshots |
| `weather-server` | `9003` | Evaluation des conditions meteo |
| `map-server` | `9004` | Carte, secteurs et exploration frontier |
| `mission-server` | `9005` | Persistance SQLite des missions |

## Prerequis

- Rust stable avec Cargo
- Un drone ou simulateur MAVLink accessible en UDP
- Optionnel : `OPENROUTER_API_KEY` pour le mode LLM reel
- Optionnel : `OPENWEATHER_API_KEY` pour les conditions meteo reelles

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Installation

```bash
git clone https://github.com/Yusufibin/drone-explorer-rs.git
cd drone-explorer-rs
cargo build --release
```

## Configuration

La configuration principale se trouve dans [`config/config.yaml`](config/config.yaml).

Variables d'environnement utiles :

```bash
export OPENROUTER_API_KEY="..."
export OPENWEATHER_API_KEY="..."
export RUST_LOG=info
```

Sans cle OpenRouter, l'agent peut basculer sur le mode de simulation integre pour executer une mission hors ligne.

## Demarrage

Lancez les serveurs MCP dans des terminaux separes :

```bash
cargo run --release --bin flight-server -- --port 9001
cargo run --release --bin vision-server -- --port 9002
cargo run --release --bin weather-server -- --port 9003
cargo run --release --bin map-server -- --port 9004
cargo run --release --bin mission-server -- --port 9005
```

Puis lancez l'agent :

```bash
cargo run --release --bin agent -- \
  --mission "Explore le secteur Nord-Est, cartographie la zone et signale les obstacles."
```

## Exemple de mission

```bash
cargo run --release --bin agent -- \
  --config config/config.yaml \
  --mission "Explore le parc de Vincennes, repere les cibles industrielles et consigne les decouvertes."
```

Les resultats runtime sont stockes localement :

- `data/mission.db` pour les missions, evenements, waypoints et decouvertes
- `snapshots/` pour les captures issues du flux video
- `reports/` pour les sorties de mission si activees

Ces dossiers/fichiers sont ignores par Git pour eviter de publier des donnees de vol ou de test.

## Securite

Ce projet pilote une architecture de station au sol et doit etre teste en simulateur avant tout usage reel. Verifiez toujours :

- la reglementation locale de vol,
- la geofence et l'altitude maximale,
- les failsafes batterie, meteo et liaison sol,
- le comportement RTL avant une mission autonome.

## Topics GitHub suggeres

`rust`, `drone`, `mavlink`, `mcp`, `llm-agent`, `autonomous-drone`, `ground-station`, `tokio`, `json-rpc`, `sqlite`, `computer-vision`, `robotics`, `uav`, `react-agent`, `openrouter`

## Licence

Distribue sous licence MIT.
