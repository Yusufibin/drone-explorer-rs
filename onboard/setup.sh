#!/bin/bash
# =============================================================
# setup.sh — Installation du Raspberry Pi Zero 2W comme pont WiFi
# =============================================================
# Ce script configure le Pi Zero pour :
#   1. Créer un hotspot WiFi
#   2. Relayer MAVLink (UART FC → UDP sol)
#   3. Streamer la caméra (libcamera + GStreamer → RTSP/UDP)
# =============================================================

set -euo pipefail

echo "=== Installation du pont drone (Pi Zero 2W) ==="

# Mise à jour système
sudo apt update && sudo apt upgrade -y

# ── 1. Outils réseau & hotspot ──
echo ">>> Installation hostapd + dnsmasq..."
sudo apt install -y hostapd dnsmasq

# Configuration hotspot
sudo tee /etc/hostapd/hostapd.conf > /dev/null <<EOF
interface=wlan0
driver=nl80211
ssid=DroneExplorer
hw_mode=g
channel=7
wmm_enabled=0
macaddr_acl=0
auth_algs=1
ignore_broadcast_ssid=0
wpa=2
wpa_passphrase=drone2024secure
wpa_key_mgmt=WPA-PSK
wpa_pairwise=TKIP
rsn_pairwise=CCMP
EOF

sudo tee /etc/dnsmasq.conf > /dev/null <<EOF
interface=wlan0
dhcp-range=192.168.4.2,192.168.4.20,255.255.255.0,24h
EOF

# IP statique pour wlan0
sudo tee -a /etc/dhcpcd.conf > /dev/null <<EOF

interface wlan0
static ip_address=192.168.4.1/24
nohook wpa_supplicant
EOF

sudo systemctl unmask hostapd
sudo systemctl enable hostapd
sudo systemctl enable dnsmasq

# ── 2. MAVLink Router ──
echo ">>> Installation mavlink-router..."
sudo apt install -y git meson ninja-build pkg-config gcc g++ python3
git clone https://github.com/mavlink-router/mavlink-router.git /tmp/mavlink-router
cd /tmp/mavlink-router
git submodule update --init --recursive
meson setup build .
ninja -C build
sudo ninja -C build install
cd ~

# Configuration mavlink-router
sudo mkdir -p /etc/mavlink-router
sudo tee /etc/mavlink-router/main.conf > /dev/null <<EOF
[General]
TcpServerPort=0
ReportStats=false

[UartEndpoint flight_controller]
Device=/dev/serial0
Baud=57600

[UdpEndpoint ground_station]
Mode=Normal
Address=192.168.4.2
Port=14550
EOF

# Service systemd pour mavlink-router
sudo tee /etc/systemd/system/mavlink-router.service > /dev/null <<EOF
[Unit]
Description=MAVLink Router
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/mavlink-routerd
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable mavlink-router

# ── 3. Caméra + GStreamer ──
echo ">>> Installation libcamera + GStreamer..."
sudo apt install -y libcamera-apps gstreamer1.0-tools \
    gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav

# Service systemd pour le stream vidéo
sudo tee /etc/systemd/system/drone-stream.service > /dev/null <<EOF
[Unit]
Description=Drone Camera Stream
After=network.target

[Service]
Type=simple
ExecStart=/home/pi/drone/stream.sh
Restart=always
RestartSec=3
User=pi

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable drone-stream

echo ""
echo "=== Installation terminée ==="
echo "Redémarrez le Pi Zero pour activer le hotspot WiFi."
echo "  sudo reboot"
echo ""
echo "Après redémarrage :"
echo "  - SSID WiFi : DroneExplorer"
echo "  - IP drone  : 192.168.4.1"
echo "  - MAVLink   : udp://192.168.4.2:14550"
echo "  - Stream    : udp://192.168.4.2:5600"
