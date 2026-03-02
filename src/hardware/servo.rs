//! DS-S006L servo service
//!
//! Uses ESP32-C3 LEDC hardware PWM at 50 Hz to reduce jitter.

use esp_idf_hal::{ledc, peripheral::Peripheral, prelude::*};
use log::{info, warn};
use std::sync::Mutex;

use crate::system::config::{
    SERVO_ANGLE_MAX, SERVO_ANGLE_MIN, SERVO_PULSE_MAX_US, SERVO_PULSE_MIN_US,
};

const SERVO_PWM_FREQ_HZ: u32 = 50;
const SERVO_PERIOD_US: u32 = 20_000;

pub struct ServoService {
    angle: Mutex<u16>,
    pwm: Mutex<ledc::LedcDriver<'static>>,
    max_duty: u32,
}

impl ServoService {
    /// Create a new servo using a **shared** LEDC timer (same timer for multiple servos).
    /// Use this when driving two servos so they share one timer and avoid channel/timer conflicts.
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
        info!("Initializing DS-S006L servo on GPIO{}", gpio_pin);

        let mut pwm = ledc::LedcDriver::new(channel, timer, pin)?;
        let max_duty = pwm.get_max_duty();

        let initial_angle = 90u16;
        let initial_pulse = angle_to_pulse_us(initial_angle);
        let initial_duty = pulse_to_duty(initial_pulse, max_duty);
        pwm.set_duty(initial_duty)?;

        Ok(Self {
            angle: Mutex::new(initial_angle),
            pwm: Mutex::new(pwm),
            max_duty,
        })
    }

    /// Create a new servo on the given LEDC channel/timer and GPIO pin (owns its own timer).
    /// Prefer `new_with_shared_timer` when using two servos to share one timer.
    pub fn new<C, T, PIN>(
        channel: impl Peripheral<P = C> + 'static,
        timer: impl Peripheral<P = T> + 'static,
        pin: impl Peripheral<P = PIN> + 'static,
        gpio_pin: u8,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        T: ledc::LedcTimer + 'static,
        C: ledc::LedcChannel<SpeedMode = <T as ledc::LedcTimer>::SpeedMode> + 'static,
        PIN: esp_idf_hal::gpio::OutputPin,
    {
        info!("Initializing DS-S006L servo on GPIO{}", gpio_pin);

        let timer_cfg = ledc::config::TimerConfig::new()
            .frequency(SERVO_PWM_FREQ_HZ.Hz())
            .resolution(ledc::config::Resolution::Bits12);
        let timer_driver = ledc::LedcTimerDriver::new(timer, &timer_cfg)?;
        let leaked_timer = Box::leak(Box::new(timer_driver));

        let mut pwm = ledc::LedcDriver::new(channel, &*leaked_timer, pin)?;
        let max_duty = pwm.get_max_duty();

        let initial_angle = 90u16;
        let initial_pulse = angle_to_pulse_us(initial_angle);
        let initial_duty = pulse_to_duty(initial_pulse, max_duty);
        pwm.set_duty(initial_duty)?;

        Ok(Self {
            angle: Mutex::new(initial_angle),
            pwm: Mutex::new(pwm),
            max_duty,
        })
    }

    pub fn set_angle(&self, angle: u16) -> Result<(), String> {
        let clamped = angle.clamp(SERVO_ANGLE_MIN, SERVO_ANGLE_MAX);
        let pulse = angle_to_pulse_us(clamped);
        let duty = pulse_to_duty(pulse, self.max_duty);

        info!(
            "Servo angle request={} clamped={} pulse={}us duty={}",
            angle, clamped, pulse, duty
        );

        self.pwm
            .lock()
            .map_err(|e| format!("Failed to lock servo pwm: {}", e))?
            .set_duty(duty)
            .map_err(|e| {
                warn!("Failed to set servo duty: {}", e);
                format!("Failed to set servo duty: {}", e)
            })?;

        self.angle
            .lock()
            .map_err(|e| format!("Failed to lock servo angle: {}", e))
            .map(|mut a| *a = clamped)
    }

    pub fn get_angle(&self) -> u16 {
        self.angle.lock().map(|a| *a).unwrap_or(90)
    }
}

fn angle_to_pulse_us(angle: u16) -> u32 {
    let a = angle.clamp(SERVO_ANGLE_MIN, SERVO_ANGLE_MAX) as u32;
    let range = (SERVO_ANGLE_MAX - SERVO_ANGLE_MIN) as u32;
    let pulse_span = SERVO_PULSE_MAX_US - SERVO_PULSE_MIN_US;
    SERVO_PULSE_MIN_US + (pulse_span * a / range.max(1))
}

fn pulse_to_duty(pulse_us: u32, max_duty: u32) -> u32 {
    max_duty.saturating_mul(pulse_us) / SERVO_PERIOD_US
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angle_pulse_mapping_matches_servo_range() {
        assert_eq!(angle_to_pulse_us(0), 500);
        assert_eq!(angle_to_pulse_us(150), 1500);
        assert_eq!(angle_to_pulse_us(300), 2500);
    }

    #[test]
    fn angle_is_clamped() {
        assert_eq!(angle_to_pulse_us(999), 2500);
        assert_eq!(angle_to_pulse_us(0), 500);
    }
}
