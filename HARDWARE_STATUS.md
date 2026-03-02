# Hardware Integration Status

## Current Status
The hardware integration code structure is complete but **temporarily disabled** due to ESP32 peripheral ownership complexity. The captive portal works perfectly without hardware.

## What's Implemented
1. ✅ Hardware module structure (`src/hardware/`)
2. ✅ VL53L0X sensor service (`vl53l0x.rs`)
3. ✅ Motor control service (`motor.rs`)
4. ✅ Distance-to-speed controller (`controller.rs`)
5. ✅ Web API endpoints (`/api/status`, `/api/distance`, `/api/motor`)
6. ✅ Real-time web interface with distance & speed display
7. ✅ System configuration for hardware pins

## Known Issues
1. **LEDC Timer/Channel Speed Mode Mismatch**: The LedcTimer and LedcChannel generic parameters require matching SpeedMode, which is complex with `impl Peripheral` trait bounds
2. **Peripheral Ownership**: ESP32 peripherals can only be taken once. Current architecture needs refactoring to:
   - Either take hardware peripherals BEFORE WiFi modem
   - Or use `unsafe` peripheral cloning correctly with `&mut` references

## Next Steps to Enable Hardware

### Option 1: Refactor Peripheral Ownership (Recommended)
```rust
// In ButterflySystem::new()
// Take hardware peripherals FIRST
let i2c0 = peripherals.i2c0;
let gpio3 = peripherals.pins.gpio3;
let gpio6 = peripherals.pins.gpio6;
let gpio7 = peripherals.pins.gpio7;
let ledc_timer = peripherals.ledc.timer0;
let ledc_channel = peripherals.ledc.channel0;

// Initialize hardware with owned peripherals
let (sensor, motor, controller) = Self::init_hardware(
    i2c0, gpio6, gpio7,
    ledc_timer, ledc_channel, gpio3
)?;

// THEN take modem for WiFi
let wifi = WifiService::new(peripherals.modem, sys_loop)?;
```

### Option 2: Use Concrete Types Instead of `impl Peripheral`
```rust
// In motor.rs - use concrete types
pub fn new(
    timer: TIMER0,
    channel: CHANNEL0,
    pin: Gpio3<Unknown>,
) -> Result<Self, Box<dyn std::error::Error>> {
    // Now speed modes will match
}
```

### Option 3: Feature Flag for Hardware
Add Cargo feature flag to compile hardware support only when explicitly enabled:
```toml
[features]
hardware = []
```

## Testing Without Hardware
The system runs perfectly as a captive portal without hardware. Connect to "butterfly" WiFi and the portal auto-opens.

## Hardware Wiring (When Ready)
```
ESP32-C3 Connections:
├── GPIO3 (D3) → MOSFET Gate (PWM motor control)
├── GPIO6 (D6) → VL53L0X SDA (I2C data)
├── GPIO7 (D7) → VL53L0X SCL (I2C clock)
├── 3V3 → VL53L0X VCC
└── GND → VL53L0X GND, MOSFET Source

MOSFET Circuit:
├── Gate → GPIO3
├── Drain → N20 Motor (-)
├── Source → GND

N20 Motor:
├── (+) → 3.7V Battery (+)
└── (-) → MOSFET Drain
```

## API Endpoints (Ready When Hardware Enabled)
- `GET /api/status` - Returns `{"distance": 123, "speed": 45}`
- `GET /api/distance` - Returns `{"distance": 123}`
- `GET /api/motor` - Returns `{"speed": 45}`

The web interface at `http://192.168.71.1/` will automatically show real-time sensor data once hardware is enabled.
