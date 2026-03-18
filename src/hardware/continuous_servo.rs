//! Continuous-rotation servo service (e.g. 360deg SG90 continuous).
//!
//! Control convention:
//! - cmd range: `SERVO2_CMD_MIN..=SERVO2_CMD_MAX` (default 0..180)
//! - cmd == neutral (default 90): stop
//! - cmd > neutral: forward, and farther from neutral => faster
//! - cmd < neutral: reverse, and farther from neutral => faster

use esp_idf_hal::{ledc, peripheral::Peripheral};
use log::{info, warn};
use std::sync::Mutex;

use crate::system::config::{
    SERVO2_CMD_MAX, SERVO2_CMD_MIN, SERVO2_CMD_NEUTRAL, SERVO_PULSE_MAX_US, SERVO_PULSE_MIN_US,
};

const SERVO_PERIOD_US: u32 = 20_000;

pub struct ContinuousServoService {
    cmd: Mutex<u16>,
    pwm: Mutex<ledc::LedcDriver<'static>>,
    max_duty: u32,
}

impl ContinuousServoService {
    /// Create a new continuous servo using a **shared** LEDC timer.
    ///
    /// When driving two servos, using a shared timer avoids timer/channel conflicts.
    pub fn new_with_shared_timer<C, T, PIN>(
        channel: impl Peripheral<P = C> + 'static,
        timer: &'static ledc::LedcTimerDriver<'static, T>,
        pin: impl Peripheral<P = PIN> + 'static,
        gpio_pin: u8,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        T: ledc::LedcTimer + 'static,
        C: ledc::LedcChannel<SpeedMode = <T as ledc::LedcTimer>::SpeedMode> + 'static,
        PIN: esp_idf_hal::gpio::OutputPin,
    {
        info!(
            "Initializing continuous servo on GPIO{} (cmd neutral={})",
            gpio_pin, SERVO2_CMD_NEUTRAL
        );

        let mut pwm = ledc::LedcDriver::new(channel, timer, pin)?;
        let max_duty = pwm.get_max_duty();

        let initial_cmd = SERVO2_CMD_NEUTRAL;
        let initial_pulse = cmd_to_pulse_us(initial_cmd);
        let initial_duty = pulse_to_duty(initial_pulse, max_duty);
        pwm.set_duty(initial_duty)?;

        Ok(Self {
            cmd: Mutex::new(initial_cmd),
            pwm: Mutex::new(pwm),
            max_duty,
        })
    }

    /// Set continuous servo command (0..180). 90 means stop.
    pub fn set_angle(&self, cmd: u16) -> Result<(), String> {
        // Keep method name `set_angle` for minimal wiring changes in the project.
        let clamped = cmd.clamp(SERVO2_CMD_MIN, SERVO2_CMD_MAX);
        let pulse = cmd_to_pulse_us(clamped);
        let duty = pulse_to_duty(pulse, self.max_duty);

        info!(
            "ContinuousServo cmd request={} clamped={} pulse={}us duty={}",
            cmd, clamped, pulse, duty
        );

        self.pwm
            .lock()
            .map_err(|e| format!("Failed to lock continuous servo pwm: {}", e))?
            .set_duty(duty)
            .map_err(|e| {
                warn!("Failed to set continuous servo duty: {}", e);
                format!("Failed to set continuous servo duty: {}", e)
            })?;

        self.cmd
            .lock()
            .map_err(|e| format!("Failed to lock continuous servo cmd: {}", e))
            .map(|mut c| *c = clamped)
    }

    /// Get current continuous servo command (0..180).
    pub fn get_angle(&self) -> u16 {
        self.cmd
            .lock()
            .map(|c| *c)
            .unwrap_or(SERVO2_CMD_NEUTRAL)
    }
}

fn cmd_to_pulse_us(cmd: u16) -> u32 {
    // Linear mapping:
    // - cmd == SERVO2_CMD_MIN => SERVO_PULSE_MIN_US
    // - cmd == SERVO2_CMD_NEUTRAL => SERVO_PULSE_MIN_US + 50% => typically 1500us
    // - cmd == SERVO2_CMD_MAX => SERVO_PULSE_MAX_US
    let c = cmd.clamp(SERVO2_CMD_MIN, SERVO2_CMD_MAX) as u32;
    let range = (SERVO2_CMD_MAX - SERVO2_CMD_MIN) as u32;
    let pulse_span = SERVO_PULSE_MAX_US - SERVO_PULSE_MIN_US;
    SERVO_PULSE_MIN_US + (pulse_span * (c - SERVO2_CMD_MIN as u32) / range.max(1))
}

fn pulse_to_duty(pulse_us: u32, max_duty: u32) -> u32 {
    max_duty.saturating_mul(pulse_us) / SERVO_PERIOD_US
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_pulse_mapping_matches_continuous_convention() {
        // With defaults:
        // SERVO_PULSE_MIN_US=500, SERVO_PULSE_MAX_US=2500, cmd 0..180 => neutral at 90 => 1500us
        assert_eq!(cmd_to_pulse_us(0), 500);
        assert_eq!(cmd_to_pulse_us(SERVO2_CMD_NEUTRAL), 1500);
        assert_eq!(cmd_to_pulse_us(SERVO2_CMD_MAX), 2500);
    }
}

