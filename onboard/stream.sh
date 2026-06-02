#!/bin/bash
# =============================================================
# stream.sh — Lancement du stream vidéo HD H.264 depuis le drone
# =============================================================
# Ce script capture la caméra IMX477 et la streame via GStreamer
# vers l'adresse IP de la station au sol sur le port 5600.
# =============================================================

set -euo pipefail

# IP de la station au sol (GCS)
GCS_IP="192.168.4.2"
GCS_PORT=5600

echo "Lancement du stream vidéo H.264 vers ${GCS_IP}:${GCS_PORT}..."

# Option 1 : Utilisation de libcamera-vid (natif sur les nouveaux kernels)
if command -v libcamera-vid &> /dev/null; then
    libcamera-vid -t 0 --inline --listen \
        -o "udp://${GCS_IP}:${GCS_PORT}" \
        --width 1280 --height 720 --framerate 30
else
    # Option 2 : Fallback GStreamer
    rpicam-vid -t 0 -n --inline -o - | \
        gst-launch-1.0 fdsrc ! h264parse ! rtph264pay ! \
        udpsink host="${GCS_IP}" port="${GCS_PORT}"
fi
