#!/usr/bin/env python3
"""
BLE Simulator - Mimics NE101 camera BLE GATT server for frontend testing.

Creates a BLE peripheral with the same service/characteristic structure as the
real NE101 firmware, allowing automated testing of the web provisioning flow
without physical hardware.

Usage:
    python3 ble_simulator.py [--model NE101] [--sn CUSTOM_SN] [--netmods wifi,cat1]

Requirements:
    pip install bless
"""

import asyncio
import json
import logging
import argparse
import sys
from typing import Optional

from bless import (
    BlessServer,
    BlessGATTCharacteristic,
    GATTCharacteristicProperties,
    GATTAttributePermissions,
)

# ---------------------------------------------------------------------------
# UUIDs (must match ble-protocol.ts and ble_prov.c)
# ---------------------------------------------------------------------------

SERVICE_UUID = "9e5d1e47-5b13-4c4f-85b3-d0e6f5a7b8c9"

CHAR_DEVICE_INFO  = "9e5d1e48-5b13-4c4f-85b3-d0e6f5a7b8c9"  # Read
CHAR_NETWORK_SCAN = "9e5d1e49-5b13-4c4f-85b3-d0e6f5a7b8c9"  # Write + Notify
CHAR_CONFIG       = "9e5d1e4a-5b13-4c4f-85b3-d0e6f5a7b8c9"  # Write
CHAR_STATUS       = "9e5d1e4c-5b13-4c4f-85b3-d0e6f5a7b8c9"  # Read + Notify
CHAR_APPLY        = "9e5d1e4d-5b13-4c4f-85b3-d0e6f5a7b8c9"  # Write

# ---------------------------------------------------------------------------
# Simulated device state
# ---------------------------------------------------------------------------

class SimulatedDevice:
    """Holds the simulated NE101 device state."""

    def __init__(self, model: str = "NE101", sn: str = "", netmods: list[str] = None):
        self.model = model
        self.sn = sn or "SIM000AABB"
        self.fw = "1.2.0-sim"
        self.netmods = netmods or ["wifi"]

        # Received configs
        self.net_config: Optional[dict] = None
        self.mqtt_config: Optional[dict] = None

        # Provisioning state machine
        self.step = "idle"
        self.error = ""
        self.ip = ""
        self.net_type = ""

    def device_info_json(self) -> str:
        return json.dumps({
            "model": self.model,
            "sn": self.sn,
            "fw": self.fw,
            "netmod": self.netmods[0] if self.netmods else "",
            "supported_netmods": self.netmods,
        })

    def status_json(self) -> str:
        return json.dumps({
            "step": self.step,
            "error": self.error,
            "ip": self.ip,
            "net_type": self.net_type,
        })

    def reset(self):
        """Reset provisioning state for next connection."""
        self.step = "idle"
        self.error = ""
        self.ip = ""
        self.net_type = ""
        self.net_config = None
        self.mqtt_config = None


# ---------------------------------------------------------------------------
# BLE Server
# ---------------------------------------------------------------------------

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger("ble-sim")


class BleSimulator:
    """BLE GATT server simulating an NE101 camera device."""

    def __init__(self, device: SimulatedDevice, advertise_name: str = "", fail_at: str = ""):
        self.device = device
        self.advertise_name = advertise_name or f"NE101-{device.sn[-6:]}"
        self.fail_at = fail_at  # Step at which to simulate failure
        self.server: Optional[BlessServer] = None
        self._apply_task: Optional[asyncio.Task] = None

    async def start(self):
        """Create and start the BLE server."""
        self.server = BlessServer(name=self.advertise_name)
        self.server.read_request_func = self._on_read
        self.server.write_request_func = self._on_write

        # Add service with all characteristics
        await self.server.add_new_service(SERVICE_UUID)

        # Device Info: Read
        await self.server.add_new_characteristic(
            SERVICE_UUID, CHAR_DEVICE_INFO,
            GATTCharacteristicProperties.read,
            self.device.device_info_json().encode(),
            GATTAttributePermissions.readable,
        )

        # Network Scan: Write + Notify (no initial value on macOS)
        await self.server.add_new_characteristic(
            SERVICE_UUID, CHAR_NETWORK_SCAN,
            GATTCharacteristicProperties.write |
            GATTCharacteristicProperties.notify,
            None,
            GATTAttributePermissions.writeable,
        )

        # Config: Write
        await self.server.add_new_characteristic(
            SERVICE_UUID, CHAR_CONFIG,
            GATTCharacteristicProperties.write,
            None,
            GATTAttributePermissions.writeable,
        )

        # Status: Read + Notify (no initial value — read callback provides it)
        await self.server.add_new_characteristic(
            SERVICE_UUID, CHAR_STATUS,
            GATTCharacteristicProperties.read |
            GATTCharacteristicProperties.notify,
            None,
            GATTAttributePermissions.readable,
        )

        # Apply: Write
        await self.server.add_new_characteristic(
            SERVICE_UUID, CHAR_APPLY,
            GATTCharacteristicProperties.write,
            None,
            GATTAttributePermissions.writeable,
        )

        await self.server.start(prioritize_local_name=False)
        log.info("BLE simulator started — advertising as '%s'", self.advertise_name)
        log.info("Service UUID: %s", SERVICE_UUID)
        log.info("Device: model=%s sn=%s fw=%s netmods=%s",
                 self.device.model, self.device.sn, self.device.fw, self.device.netmods)

    async def stop(self):
        if self._apply_task and not self._apply_task.done():
            self._apply_task.cancel()
        if self.server:
            await self.server.stop()
            log.info("BLE simulator stopped")

    # -----------------------------------------------------------------------
    # GATT callbacks
    # -----------------------------------------------------------------------

    def _on_read(self, characteristic: BlessGATTCharacteristic) -> bytes:
        uuid = characteristic.uuid.upper()
        log.info("READ  %s", self._short_uuid(uuid))

        if uuid == CHAR_DEVICE_INFO.upper():
            data = self.device.device_info_json().encode()
            log.info("  -> %s", data.decode())
            return data

        if uuid == CHAR_STATUS.upper():
            data = self.device.status_json().encode()
            log.info("  -> %s", data.decode())
            return data

        return b"{}"

    def _on_write(self, characteristic: BlessGATTCharacteristic, value: bytes):
        uuid = characteristic.uuid.upper()
        payload = value.decode(errors="replace")
        log.info("WRITE %s <- %s", self._short_uuid(uuid), payload)

        try:
            data = json.loads(payload) if payload else {}
        except json.JSONDecodeError:
            log.warning("  Invalid JSON, ignoring")
            return

        if uuid == CHAR_NETWORK_SCAN.upper():
            self._handle_network_scan(data)
        elif uuid == CHAR_CONFIG.upper():
            self._handle_config(data)
        elif uuid == CHAR_APPLY.upper():
            self._handle_apply(data)
        else:
            log.warning("  Write to unexpected characteristic: %s", uuid)

    # -----------------------------------------------------------------------
    # Command handlers
    # -----------------------------------------------------------------------

    def _handle_network_scan(self, data: dict):
        """Handle network scan request (wifi scan / cat1_status)."""
        scan_type = data.get("type", data.get("action", ""))

        if scan_type == "wifi":
            log.info("  WiFi scan requested — sending fake results")
            # Simulate a WiFi scan notification
            results = json.dumps([
                {"ssid": "CAMTHINK_DEV", "rssi": -45, "auth": True, "channel": 6},
                {"ssid": "TestNet", "rssi": -67, "auth": False, "channel": 1},
                {"ssid": "NeoMind-5G", "rssi": -72, "auth": True, "channel": 11},
            ])
            asyncio.ensure_future(self._notify(CHAR_NETWORK_SCAN, results))

        elif scan_type == "cat1_status":
            log.info("  CAT.1 status requested — sending fake status")
            status = json.dumps({
                "sim_ready": True,
                "signal_level": "Good",
                "signal_dbm": -75,
                "imei": "867891011121314",
                "iccid": "89860123456789012345",
                "isp": "China Mobile",
                "network_type": "LTE",
                "register_status": "Registered",
            })
            asyncio.ensure_future(self._notify(CHAR_NETWORK_SCAN, status))

        else:
            log.warning("  Unknown scan type: %s", scan_type)

    def _handle_config(self, data: dict):
        """Handle config write (net_wifi, net_cat1, net_halow, mqtt).

        Frontend sends flat JSON like: {"type":"net_wifi","ssid":"...","password":"..."}
        """
        config_type = data.get("type", "")

        if config_type in ("net_wifi", "net_halow"):
            log.info("  %s config saved: ssid=%s",
                     config_type, data.get("ssid", ""))
            self.device.net_config = data

        elif config_type == "net_cat1":
            log.info("  CAT.1 config saved: apn=%s", data.get("apn", ""))
            self.device.net_config = data

        elif config_type == "mqtt":
            log.info("  MQTT config saved: host=%s:%s topic=%s",
                     data.get("host", ""),
                     data.get("port", ""),
                     data.get("topic_prefix", ""))
            self.device.mqtt_config = data

        else:
            log.warning("  Unknown config type: %s", config_type)

    def _handle_apply(self, data: dict):
        """Handle apply command — simulate the provisioning sequence."""
        action = data.get("action", "")
        if action != "apply":
            log.warning("  Unknown apply action: %s", action)
            return

        if self.device.step != "idle":
            log.warning("  Provisioning already in progress: %s", self.device.step)
            return

        log.info("  Starting simulated provisioning sequence...")
        # Cancel any previous apply task
        if self._apply_task and not self._apply_task.done():
            self._apply_task.cancel()
        self._apply_task = asyncio.ensure_future(self._run_provisioning())

    # -----------------------------------------------------------------------
    # Simulated provisioning sequence
    # -----------------------------------------------------------------------

    async def _run_provisioning(self):
        """
        Simulate the NE101 provisioning flow:
        net_connecting -> net_connected -> mqtt_connecting -> done

        This mimics the simplified prov_apply_task() from the firmware,
        but completes the full sequence with status notifications.
        """
        device = self.device

        # Step 1: net_connecting
        device.step = "net_connecting"
        device.net_type = device.netmods[0] if device.netmods else "wifi"
        device.error = ""
        device.ip = ""
        log.info("[PROV] -> net_connecting")
        await self._notify_status()
        await asyncio.sleep(2.0)

        # Step 2: net_connected (simulate getting IP)
        if self.fail_at == "net_connecting":
            device.step = "failed"
            device.error = "WiFi connection timeout"
            log.info("[PROV] -> FAILED at net_connecting: %s", device.error)
            await self._notify_status()
            return

        device.step = "net_connected"
        device.ip = "192.168.1.100"
        log.info("[PROV] -> net_connected (ip=%s)", device.ip)
        await self._notify_status()
        await asyncio.sleep(1.5)

        # Step 3: mqtt_connecting
        if self.fail_at == "net_connected":
            device.step = "failed"
            device.error = "MQTT broker unreachable"
            log.info("[PROV] -> FAILED at net_connected: %s", device.error)
            await self._notify_status()
            return

        device.step = "mqtt_connecting"
        log.info("[PROV] -> mqtt_connecting")
        await self._notify_status()
        await asyncio.sleep(2.0)

        # Step 4: done
        if self.fail_at == "mqtt_connecting":
            device.step = "failed"
            device.error = "MQTT auth failed"
            log.info("[PROV] -> FAILED at mqtt_connecting: %s", device.error)
            await self._notify_status()
            return

        # Step 4: done
        device.step = "done"
        log.info("[PROV] -> done — provisioning succeeded!")
        await self._notify_status()

        log.info("Config summary:")
        log.info("  Network: %s", json.dumps(device.net_config, indent=2) if device.net_config else "(none)")
        log.info("  MQTT:    %s", json.dumps(device.mqtt_config, indent=2) if device.mqtt_config else "(none)")

    # -----------------------------------------------------------------------
    # Notification helpers
    # -----------------------------------------------------------------------

    async def _notify(self, char_uuid: str, payload: str):
        """Send a BLE notification on the given characteristic."""
        if not self.server:
            return
        try:
            char = self.server.get_characteristic(char_uuid.upper())
            if char:
                # Set the value first, then push notification to subscribers
                char.value = payload.encode()
                self.server.update_value(SERVICE_UUID, char_uuid.upper())
                log.info("NOTIFY %s -> %s", self._short_uuid(char_uuid), payload)
            else:
                log.warning("Characteristic not found for notify: %s", char_uuid)
        except Exception as e:
            log.error("Notify failed: %s", e)

    async def _notify_status(self):
        """Send a status notification."""
        await self._notify(CHAR_STATUS, self.device.status_json())

    @staticmethod
    def _short_uuid(uuid: str) -> str:
        """Shorten UUID for logging (show last 4 chars before the common suffix)."""
        # All our UUIDs differ at position 32 (hex digit before -5b13...)
        if len(uuid) >= 36:
            return f"...{uuid[32:34]}"
        return uuid


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

async def main():
    parser = argparse.ArgumentParser(description="NE101 BLE Simulator")
    parser.add_argument("--model", default="NE101", help="Device model (default: NE101)")
    parser.add_argument("--sn", default="", help="Serial number (default: auto)")
    parser.add_argument("--netmods", default="wifi", help="Comma-separated network modes (default: wifi)")
    parser.add_argument("--name", default="", help="BLE advertising name (default: NE101-XXXXXX)")
    parser.add_argument("--fail-at", default="", help="Fail provisioning at step (net_connecting, net_connected, mqtt_connecting)")
    parser.add_argument("--verbose", "-v", action="store_true", help="Enable debug logging")
    args = parser.parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    netmods = [m.strip() for m in args.netmods.split(",") if m.strip()]
    device = SimulatedDevice(model=args.model, sn=args.sn, netmods=netmods)

    sim = BleSimulator(device, advertise_name=args.name, fail_at=args.fail_at)

    # Handle graceful shutdown
    loop = asyncio.get_event_loop()
    stop_event = asyncio.Event()

    def signal_handler():
        log.info("Shutting down...")
        stop_event.set()

    try:
        import signal
        for sig in (signal.SIGINT, signal.SIGTERM):
            loop.add_signal_handler(sig, signal_handler)
    except NotImplementedError:
        # Windows doesn't support add_signal_handler
        pass

    await sim.start()

    log.info("")
    log.info("Waiting for BLE connections from Chrome...")
    log.info("Open the NeoMind web UI -> Devices -> Add Device -> BLE tab")
    if args.fail_at:
        log.info("Will simulate failure at step: %s", args.fail_at)
    log.info("Press Ctrl+C to stop")
    log.info("")

    # Keep running until interrupted
    await stop_event.wait()
    await sim.stop()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nStopped.")
