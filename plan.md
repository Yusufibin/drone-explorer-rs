# Plan — Drone Autonome LLM + MCP (Ground Station Edition)

*Version adaptée : drone DIY léger · caméra HD · WiFi · cerveau au sol*

---

## 1. Vision générale

Construire un système d'**exploration et cartographie autonome** où un drone DIY basique streame sa caméra vers une station au sol, et où un **LLM orchestre** les décisions de vol via le **Model Context Protocol (MCP)**. Le drone reçoit une mission en langage naturel ("explore cette zone, cartographie les bâtiments"), l'exécute, et prend des décisions en temps réel.

### Principes fondateurs

- **Tout le cerveau est au sol** : le drone est un capteur volant. Pas d'IA embarquée.
- **LLM = décisions haut niveau** : exploration, cartographie, analyse visuelle.
- **MCP = interfaces modulaires** : chaque capacité est un serveur indépendant.
- **WiFi = lien unique** : télémétrie MAVLink + stream vidéo via le même réseau.
- **Coût minimal** : matériel < 500 €, pas de Jetson, pas de LiDAR.

---

## 2. Architecture globale

```
╔══════════════════════════════════════════════════════════════════╗
║                    STATION AU SOL (PC / Laptop)                  ║
║                                                                  ║
║  ┌─────────────────────────────────────────────────────────┐    ║
║  │                  AGENT ORCHESTRATEUR                     │    ║
║  │           LiteLLM + Boucle ReAct événementielle          │    ║
║  └──────┬──────┬──────┬──────┬──────┬──────────────────────┘    ║
║         │      │      │      │      │                            ║
║  ┌──────▼┐ ┌───▼──┐ ┌─▼────┐ ┌▼────┐ ┌▼──────┐                 ║
║  │ Vol   │ │Vision│ │Météo │ │Carte│ │Mission│  ← Serveurs MCP  ║
║  │ MCP   │ │ MCP  │ │ MCP  │ │ MCP │ │  MCP  │                  ║
║  └──────┬┘ └───┬──┘ └─┬────┘ └┬────┘ └┬──────┘                 ║
║         │      │      │       │       │                          ║
║  ┌──────▼┐ ┌───▼──┐ ┌─▼────┐ ┌▼────┐ ┌▼──────┐                 ║
║  │MAVS-  │ │OpenCV│ │Open- │ │OSM  │ │SQLite │                  ║
║  │DK/UDP │ │+YOLO │ │Weath │ │Folium│ │/JSON  │                 ║
║  └──────┬┘ └──────┘ └──────┘ └─────┘ └───────┘                 ║
║         │                                                        ║
║         │  MAVLink UDP + RTSP/MJPEG stream                       ║
╚═════════╪════════════════════════════════════════════════════════╝
          │  WiFi 2.4 / 5 GHz
╔═════════╪════════════════════════════════════════════════════════╗
║         │             DRONE DIY                                   ║
║  ┌──────▼──────────────────────────────────┐                     ║
║  │  Flight Controller (ArduPilot/PX4)      │                     ║
║  │  WiFi Module (ESP32 / Raspberry Pi Zero) │                     ║
║  │  Caméra HD (IMX477 / GoPro / RunCam)    │                     ║
║  │  GPS standard (M8N / M9N)               │                     ║
║  └─────────────────────────────────────────┘                     ║
╚════════════════════════════════════════════════════════════════╝
```

### Ce que fait le drone
- Voler, stabiliser, suivre des waypoints GPS
- Streamer la vidéo en WiFi (RTSP ou MJPEG)
- Envoyer sa télémétrie via MAVLink over UDP
- Exécuter des commandes de vol reçues via MAVLink

### Ce que fait la station au sol
- Recevoir le stream vidéo et l'analyser (vision)
- Raisonner avec le LLM sur les décisions de mission
- Envoyer des commandes de vol au drone (waypoints, modes)
- Construire la carte en temps réel
- Journaliser la mission

---

## 3. Architecture à 3 vitesses — Le LLM n'est jamais dans la boucle critique

Même sans companion computer, la séparation est stricte :

```
┌────────────────────────────────────────────────────────────┐
│ NIVEAU 3 — STRATÉGIQUE (LLM au sol)          1-5 secondes  │
│                                                             │
│ "Cette zone est explorée, je vais vers le secteur nord"    │
│ "Objet intéressant détecté, je tourne autour"              │
│ "Batterie faible, je rentre à la base"                     │
│                                                             │
│ Modèle : Claude Sonnet / GPT-4o / DeepSeek via OpenRouter  │
└───────────────────────────┬─────────────────────────────────┘
                            │ goto(lat, lon) · set_mode(RTL)
                            │ [MAVLink UDP over WiFi]
┌───────────────────────────▼─────────────────────────────────┐
│ NIVEAU 2 — TACTIQUE (ArduPilot/PX4 + scripts Python)  50ms  │
│                                                             │
│ Suivi de waypoint · Mode GUIDED · Loiter automatique        │
│ Replanification simple si waypoint inatteignable            │
│ Scripts Python au sol (MAVSDK) : logique de mission         │
└───────────────────────────┬─────────────────────────────────┘
                            │ commandes attitude/position
┌───────────────────────────▼─────────────────────────────────┐
│ NIVEAU 1 — RÉACTIF (firmware ArduPilot/PX4)       < 1 ms   │
│                                                             │
│ PID · Stabilisation · Mixers · ESC                          │
│ Le drone vole TOUT SEUL même si le WiFi coupe               │
│ Failsafe : RTH automatique si perte de liaison              │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. Hardware — Budget accessible

### 4.1 Liste du matériel

| Composant | Référence recommandée | Prix estimé | Rôle |
|-----------|----------------------|-------------|------|
| Frame | 5" freestyle ou 7" long range | 30–70 € | Structure |
| Flight Controller | Matek H743 / SpeedyBee F405 | 40–70 € | Vol, stabilisation |
| ESC | 4-en-1 30-45A | 35–55 € | Moteurs |
| Moteurs | 2306 / 2207 (4x) | 40–70 € | Propulsion |
| WiFi link | Raspberry Pi Zero 2W + hostapd | 20–25 € | MAVLink UDP + stream vidéo |
| Caméra | Raspberry Pi HQ Camera (IMX477) | 50–70 € | Stream HD 1080p/4K |
| GPS | GPS M9N ou M8N | 25–40 € | Position, waypoints |
| Batterie | LiPo 4S 3000–4000 mAh | 30–50 € | Autonomie ~15-20 min |
| Props, câbles, divers | — | 20–30 € | — |
| **Total drone** | | **~290–480 €** | |

> **Station au sol** : ton PC ou laptop existant. Aucun achat supplémentaire.

### 4.2 Schéma de communication

```
Drone                          Station au sol (PC)
  │                                    │
  │  WiFi 2.4/5 GHz (hotspot drone)   │
  │  ─────────────────────────────>   │
  │  MAVLink UDP :14550               │  → MAVSDK Python → MCP flight
  │  RTSP stream rtsp://10.0.0.1:8554 │  → OpenCV → MCP vision
```

Le **Raspberry Pi Zero 2W** embarqué joue le rôle de **pont WiFi uniquement** :
- Crée un hotspot WiFi
- Relaie MAVLink entre le FC (UART) et le PC (UDP)
- Streame la caméra (libcamera + GStreamer → RTSP)
- **N'exécute aucune IA** — juste pont réseau

---

## 5. Stack technique

### 5.1 Langages & Frameworks

| Technologie | Usage |
|-------------|-------|
| Python 3.12+ | Orchestrateur, serveurs MCP, vision |
| fastmcp | Framework MCP serveur (Python) |
| MAVSDK Python | Communication MAVLink avec le FC |
| LiteLLM | Abstraction LLM multi-modèles |
| OpenRouter | API unifiée ~200 LLMs |
| OpenCV + YOLOv8 | Traitement vidéo + détection |
| Folium / OpenStreetMap | Cartographie, génération de carte |
| SQLite | Journalisation de mission |
| GStreamer / libcamera | Stream vidéo depuis le Pi Zero |

### 5.2 Structure du projet

```
drone-explorer/
├── agent/
│   ├── orchestrator.py       # Boucle ReAct principale
│   ├── prompter.py           # Construction des prompts
│   ├── memory.py             # Historique de mission
│   └── config.py             # Config LLM, modèles, OpenRouter
│
├── servers/
│   ├── flight/
│   │   ├── server.py         # Serveur FastMCP
│   │   └── mavsdk_bridge.py  # MAVSDK → MAVLink UDP
│   │
│   ├── vision/
│   │   ├── server.py
│   │   ├── stream_reader.py  # Lecture RTSP / MJPEG
│   │   └── detector.py       # YOLOv8 + OpenCV
│   │
│   ├── weather/
│   │   ├── server.py
│   │   └── openweather.py
│   │
│   ├── map/
│   │   ├── server.py
│   │   ├── builder.py        # Construction carte Folium/OSM
│   │   └── explorer.py       # Logique frontier exploration
│   │
│   └── mission/
│       ├── server.py
│       └── db.py             # SQLite logs
│
├── onboard/                  # Code pour le Pi Zero 2W
│   ├── setup.sh              # Install libcamera, GStreamer, mavlink-router
│   ├── stream.sh             # Lance le stream RTSP
│   └── mavlink_bridge.sh     # UART FC → UDP sol
│
├── config/
│   ├── drone.yaml            # Paramètres physique drone
│   ├── models.yaml           # Routage LLM par type de tâche
│   └── mcp_servers.yaml      # Config serveurs MCP
│
├── tests/
│   ├── test_orchestrator.py
│   ├── test_flight.py        # SITL ArduPilot
│   └── test_vision.py        # Frames de test
│
├── requirements.txt
└── README.md
```

---

## 6. Serveurs MCP

### 6.1 flight-mcp

```python
from fastmcp import FastMCP
from .mavsdk_bridge import MAVSDKBridge

mcp = FastMCP("drone-flight")
bridge = MAVSDKBridge("udp://:14550")  # WiFi UDP

@mcp.tool()
async def takeoff(altitude: float = 10.0) -> bool:
    """Décolle à l'altitude spécifiée (mètres)"""
    return await bridge.takeoff(altitude)

@mcp.tool()
async def goto(lat: float, lon: float, alt: float = 20.0) -> bool:
    """Se déplace vers une position GPS"""
    return await bridge.goto(lat, lon, alt)

@mcp.tool()
async def land() -> bool:
    """Atterrissage immédiat à la position actuelle"""
    return await bridge.land()

@mcp.tool()
async def rtl() -> bool:
    """Return-To-Launch"""
    return await bridge.rtl()

@mcp.tool()
async def get_telemetry() -> dict:
    """Position, altitude, vitesse, cap, mode de vol"""
    return await bridge.get_telemetry()

@mcp.tool()
async def get_battery() -> dict:
    """Batterie : pourcentage, voltage, temps restant estimé"""
    return await bridge.get_battery()

@mcp.tool()
async def loiter(duration_s: int = 10) -> bool:
    """Tourne en rond à la position actuelle pendant N secondes"""
    return await bridge.loiter(duration_s)

@mcp.tool()
async def set_speed(speed_m_s: float = 5.0) -> bool:
    """Vitesse de croisière en m/s (max 15 m/s)"""
    return await bridge.set_speed(speed_m_s)
```

### 6.2 vision-mcp

```python
@mcp.tool()
async def analyze_frame() -> dict:
    """
    Capture le frame courant depuis le stream RTSP et retourne :
    - description textuelle du terrain survolé
    - objets détectés (YOLO) avec confiance
    - type de terrain (urbain, végétation, eau, route...)
    """

@mcp.tool()
async def detect_objects(min_confidence: float = 0.5) -> list[dict]:
    """Détecte tous les objets sur le frame courant (YOLOv8)"""

@mcp.tool()
async def get_snapshot(save: bool = True) -> str:
    """Capture et sauvegarde le frame courant → chemin fichier"""

@mcp.tool()
async def check_landing_zone() -> dict:
    """Analyse si la zone sous le drone est safe pour atterrir"""
```

### 6.3 map-mcp

```python
@mcp.tool()
async def add_waypoint(lat: float, lon: float,
                       label: str, photo_path: str | None = None) -> bool:
    """Ajoute un point d'intérêt à la carte avec photo optionnelle"""

@mcp.tool()
async def get_unexplored_sector() -> dict:
    """
    Retourne le prochain secteur à explorer selon la logique frontier :
    { lat, lon, priority, reason }
    """

@mcp.tool()
async def get_coverage_percent() -> float:
    """Pourcentage de la zone cible déjà couverte"""

@mcp.tool()
async def export_map(format: str = "html") -> str:
    """Exporte la carte Folium en HTML ou image PNG"""
```

### 6.4 weather-mcp

```python
@mcp.tool()
async def get_conditions() -> dict:
    """Vent, visibilité, pluie, température via OpenWeatherMap"""

@mcp.tool()
async def is_safe_to_fly() -> dict:
    """
    Verdict binaire + raison :
    { safe: bool, reason: str, max_altitude: int }
    """
```

### 6.5 mission-mcp

```python
@mcp.tool()
async def log_finding(lat: float, lon: float,
                      description: str, photo_path: str) -> bool:
    """Enregistre une découverte dans la base de données"""

@mcp.tool()
async def get_mission_status() -> dict:
    """Résumé : waypoints visités, découvertes, % mission, temps écoulé"""

@mcp.tool()
async def complete_mission(summary: str) -> bool:
    """Clôture la mission et génère le rapport final"""
```

---

## 7. Agent Orchestrateur — Boucle ReAct

### 7.1 Boucle événementielle

```python
async def orchestrator_loop():
    while mission_active:
        # 1. Attendre un événement
        event = await event_queue.get()
        # Événements possibles :
        #   "waypoint_reached"   → prochaine étape
        #   "object_detected"    → analyser et décider
        #   "battery_low"        → retour prioritaire
        #   "weather_degraded"   → adapter ou abandonner
        #   "zone_covered"       → passer au secteur suivant
        #   "periodic_tick"      → check toutes les 20s

        # 2. Collecter le contexte
        context = await collect_context()
        # → get_telemetry() + get_battery() + get_coverage_percent()

        # 3. Appeler le LLM si nécessaire
        if event.requires_reasoning:
            response = await llm.complete(
                build_prompt(mission, event, context, tools)
            )
            if response.tool_call:
                result = await mcp_client.call_tool(
                    response.tool_call.name,
                    response.tool_call.args
                )
                mission_log.append(event, response, result)

        else:
            # Réflexe bas niveau sans LLM (ex: waypoint_reached → next WP)
            await handle_reflex(event, context)
```

### 7.2 Prompt système type

```
Tu es le système de navigation d'un drone explorateur autonome.
Mission : "{mission_description}"

État actuel :
- Position GPS : {lat:.6f}, {lon:.6f} @ {alt:.1f}m
- Batterie : {battery_pct}% (~{time_remaining} min restantes)
- Vent : {wind_speed} km/h direction {wind_dir}
- Couverture zone : {coverage_pct}%
- Dernière analyse visuelle : {last_vision_result}
- Heure locale : {local_time}

Règles absolues :
1. Batterie < 25% → rtl() immédiatement, aucune exception
2. Vent > 45 km/h → rtl(), conditions dangereuses
3. Une décision = un appel d'outil. Attends le résultat.
4. Après chaque analyse visuelle intéressante → log_finding()
5. Explore systématiquement par secteurs via get_unexplored_sector()

Outils disponibles : {tools_list}

Décide la prochaine action.
```

### 7.3 Routage des modèles par tâche

| Type de décision | Modèle | Justification |
|-----------------|--------|---------------|
| Navigation (goto, waypoints) | `deepseek/deepseek-chat` | Rapide, bon marché |
| Analyse terrain + image | `anthropic/claude-sonnet-4` | Vision + raisonnement |
| Décision critique (danger, batterie) | `anthropic/claude-sonnet-4` | Fiabilité |
| Planification de mission | `anthropic/claude-opus-4` | Complexité haut niveau |

---

## 8. Code onboard — Raspberry Pi Zero 2W

Le Pi Zero fait uniquement du **bridging réseau**. Pas d'IA, pas de logique métier.

### 8.1 Bridge MAVLink (UART → UDP)

```bash
# onboard/mavlink_bridge.sh
# mavlink-router : relaie le FC vers le PC au sol via UDP
mavlink-routerd \
  -e 192.168.1.100:14550 \   # IP PC au sol, port MAVLink
  /dev/serial0:57600          # UART vers Flight Controller
```

### 8.2 Stream vidéo RTSP

```bash
# onboard/stream.sh
# GStreamer : stream la caméra IMX477 en H.264 via RTSP
libcamera-vid -t 0 --inline --listen \
  -o udp://192.168.1.100:5600 \
  --width 1920 --height 1080 --framerate 30

# Ou via rpicam-apps + GStreamer pour RTSP :
rpicam-vid -t 0 -n --inline -o - | \
  gst-launch-1.0 fdsrc ! h264parse ! rtph264pay ! \
  udpsink host=192.168.1.100 port=5600
```

### 8.3 Réception côté PC

```python
# Au sol : lecture du stream dans vision-mcp
import cv2

cap = cv2.VideoCapture("udp://0.0.0.0:5600")
# ou RTSP : cv2.VideoCapture("rtsp://10.0.0.1:8554/drone")

while True:
    ret, frame = cap.read()
    if ret:
        # frame disponible pour analyse YOLO / snapshot
        current_frame = frame
```

---

## 9. Roadmap — Phases de développement

### Phase 1 — Foundation (Semaine 1-2)

- [ ] Assembler le drone (frame, FC, ESC, moteurs)
- [ ] Configurer ArduPilot + calibrations (accéléro, boussole, radio)
- [ ] Installer mavlink-router sur le Pi Zero 2W
- [ ] Valider la communication MAVLink via WiFi (Mission Planner)
- [ ] Test de vol manuel basique
- [x] Plan d'architecture (ce document)

### Phase 2 — Stream vidéo + Communication sol (Semaine 3)

- [ ] Stream RTSP/UDP depuis le Pi Zero vers le PC
- [ ] Réception et affichage fluide sur le PC (OpenCV)
- [ ] MAVSDK Python : takeoff, goto, land, get_telemetry depuis le PC
- [ ] Vol en mode GUIDED piloté par script Python

### Phase 3 — Serveurs MCP Flight + Vision (Semaine 4-5)

- [ ] Serveur `flight-mcp` : 8 outils, testés en SITL ArduPilot
- [ ] Serveur `vision-mcp` : YOLOv8 sur le stream en temps réel
- [ ] Analyse terrain basique (classif. OpenCV)
- [ ] Tests unitaires pour chaque outil

### Phase 4 — Cartographie + Mission (Semaine 6-7)

- [ ] Serveur `map-mcp` : carte Folium, gestion des secteurs
- [ ] Logique d'exploration frontier (secteurs non couverts)
- [ ] Serveur `mission-mcp` : logs SQLite, génération rapport
- [ ] Serveur `weather-mcp` : API OpenWeather

### Phase 5 — Orchestrateur LLM (Semaine 8-9)

- [ ] Boucle ReAct avec LiteLLM + OpenRouter
- [ ] Découverte dynamique des outils MCP
- [ ] Prompt système et templates
- [ ] Routage modèles par type de tâche
- [ ] Mission complète autonome : "explore ce périmètre"

### Phase 6 — Autonomie & Hardening (Semaine 10-12)

- [ ] Gestion batterie faible → RTH automatique
- [ ] Failsafe WiFi (le drone rentre si liaison coupée > 5s)
- [ ] Dashboard web temps réel (Flask + carte Folium live)
- [ ] Missions multi-zones avec logique conditionnelle
- [ ] Rapport de mission automatique (PDF avec photos + carte)
- [ ] Tests exhaustifs + démo vidéo

---

## 10. Tests & Validation

### Simulation

- **ArduPilot SITL** : simuler le vol sans drone physique
- **Mocks MCP** : chaque serveur mocké pour tester l'orchestrator
- **Frames vidéo pré-enregistrées** : tester vision-mcp offline

```bash
# Lancer ArduPilot SITL
sim_vehicle.py -v ArduCopter --console --map

# Tester le serveur flight-mcp contre SITL
python -m pytest tests/test_flight.py -v

# Tester la vision sur frames locales
python -m pytest tests/test_vision.py --frames tests/fixtures/
```

### Métriques cibles

| Métrique | Objectif |
|----------|----------|
| Taux de mission complétée | > 85% |
| Latence décision LLM | < 3s |
| Latence commande MAVLink | < 200ms |
| Délai stream vidéo | < 500ms |
| Couverture zone par mission | > 80% |
| Failsafe WiFi déclenché | < 5s après coupure |

---

## 11. Sécurité & Failsafes

### Failsafes ArduPilot (configurés dans Mission Planner)

```yaml
# config/drone.yaml — paramètres ArduPilot
FS_BATT_ENABLE: 1          # Failsafe batterie activé
FS_BATT_VOLTAGE: 14.2      # RTH si tension < 14.2V (4S)
FS_BATT_MAH: 400           # RTH si < 400 mAh restants
FS_THR_ENABLE: 1           # Failsafe perte signal RC
FS_GCS_ENABLE: 1           # Failsafe perte GCS (WiFi)
FS_GCS_TIMEOUT: 5          # RTH si WiFi coupé > 5s
FENCE_ENABLE: 1            # Geofence activée
FENCE_TYPE: 3              # Cercle + altitude max
FENCE_RADIUS: 200          # Rayon max 200m du point de départ
FENCE_ALT_MAX: 80          # Altitude max 80m (légal FR)
```

### Dans le code

```python
# Vérifications systématiques avant chaque commande de vol
async def safe_goto(lat, lon, alt):
    battery = await get_battery()
    if battery["percent"] < 25:
        await rtl()
        raise SafetyError("Batterie < 25%, RTL déclenché")

    weather = await is_safe_to_fly()
    if not weather["safe"]:
        raise SafetyError(f"Conditions météo: {weather['reason']}")

    if alt > 80:
        raise SafetyError("Altitude max légale FR = 80m")

    return await bridge.goto(lat, lon, alt)
```

---

## 12. Budget total

### Hardware (investissement unique)

| Composant | Prix estimé |
|-----------|-------------|
| Frame 5-7" | 30–70 € |
| Flight Controller | 40–70 € |
| ESC 4-en-1 | 35–55 € |
| Moteurs x4 | 40–70 € |
| Raspberry Pi Zero 2W | 20–25 € |
| Caméra RPi HQ (IMX477) | 50–70 € |
| GPS M8N / M9N | 25–40 € |
| LiPo 4S 3000mAh | 30–50 € |
| Radio RC (optionnelle) | 30–60 € |
| Câbles, connecteurs, divers | 20–30 € |
| **Total** | **~320–540 €** |

### Coûts mensuels (opération)

| Poste | Coût estimé |
|-------|-------------|
| OpenRouter API (usage modéré) | 10–30 € |
| OpenWeather API (plan gratuit) | 0 € |
| **Total** | **~10–30 €/mois** |

---

## 13. Références

| Projet / Ressource | Élément repris |
|--------------------|----------------|
| [ArduPilot MAVSDK](https://mavsdk.mavlink.io) | Communication MAVLink Python |
| [fastmcp](https://github.com/jlowin/fastmcp) | Framework MCP serveur Python |
| [LiteLLM](https://github.com/BerriAI/litellm) | Abstraction multi-LLM |
| [OpenRouter](https://openrouter.ai) | API unifiée ~200 modèles |
| [YOLOv8 Ultralytics](https://github.com/ultralytics/ultralytics) | Détection objets temps réel |
| [Folium](https://python-visualization.github.io/folium) | Cartographie interactive Python |
| [Model Context Protocol](https://modelcontextprotocol.io) | Spécification MCP officielle |

---

## 14. Licence

MIT License. Contributions bienvenues — voir `CONTRIBUTING.md`.

---

*Document v2.0 — 13 mai 2026 — adapté architecture Ground Station*
