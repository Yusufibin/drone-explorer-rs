#!/bin/bash
# =============================================================
# mavlink_bridge.sh — Relaie le flux MAVLink du FC vers le sol
# =============================================================
# Utilise mavlink-router pour relayer la télémétrie et les
# commandes entre le port série physique et la Ground Station.
# =============================================================

set -euo pipefail

# IP de la station au sol (GCS)
GCS_IP="192.168.4.2"
GCS_PORT=14550

# Port série physique du Raspberry Pi connecté au FC (UART)
UART_DEV="/dev/serial0"
BAUD_RATE=57600

echo "Lancement du pont MAVLink : ${UART_DEV}@${BAUD_RATE} <-> ${GCS_IP}:${GCS_PORT}..."

exec mavlink-routerd \
    -e "${GCS_IP}:${GCS_PORT}" \
    "${UART_DEV}:${BAUD_RATE}"
