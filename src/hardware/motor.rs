//! DRV8833 motor service
//!
//! IN1: PWM pin for speed control
//! IN2: direction pin for forward/reverse and brake/coast

use esp_idf_hal::{
    delay::{Ets, FreeRtos},
    gpio::PinDriver,
    peripheral::Peripheral,
};
use log::info;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::system::config::{MOTOR_IN1_PIN, MOTOR_IN2_PIN, MOTOR_PWM_FREQUENCY_HZ};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotorDirection {
    Coast,
    Forward,
    Reverse,
    Brake,
}

impl MotorDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Coast => "coast",
            Self::Forward => "forward",
            Self::Reverse => "reverse",
            Self::Brake => "brake",
        }
    }
}

#[derive(Clone, Copy)]
struct MotorState {
    speed: u8,
    direction: MotorDirection,
}

pub struct MotorService {
    state: Arc<Mutex<MotorState>>,
}

const SOFT_PWM_PERIOD_US: u32 = 2000;

impl MotorService {
    pub fn new<IN1, IN2>(
        in1: impl Peripheral<P = IN1> + 'static,
        in2: impl Peripheral<P = IN2> + 'static,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        IN1: esp_idf_hal::gpio::OutputPin,
        IN2: esp_idf_hal::gpio::OutputPin,
    {
        info!(
            "Initializing DRV8833 motor on IN1=GPIO{}, IN2=GPIO{}",
            MOTOR_IN1_PIN, MOTOR_IN2_PIN
        );

        let mut in1_pin = PinDriver::output(in1)?;
        let mut in2_pin = PinDriver::output(in2)?;

        in1_pin.set_low()?;
        in2_pin.set_low()?;

        let state = Arc::new(Mutex::new(MotorState {
            speed: 0,
            direction: MotorDirection::Coast,
        }));
        let state_clone = Arc::clone(&state);

        let _pwm_hz = MOTOR_PWM_FREQUENCY_HZ.max(1);

        thread::Builder::new()
            .name("drv8833-motor".to_string())
            .spawn(move || loop {
                let snapshot = match state_clone.lock() {
                    Ok(guard) => *guard,
                    Err(_) => {
                        FreeRtos::delay_ms(10);
                        continue;
                    }
                };

                let speed = snapshot.speed.min(100) as u32;
                let on_us = SOFT_PWM_PERIOD_US * speed / 100;
                let off_us = SOFT_PWM_PERIOD_US.saturating_sub(on_us);

                match snapshot.direction {
                    MotorDirection::Coast => {
                        let _ = in1_pin.set_low();
                        let _ = in2_pin.set_low();
                        FreeRtos::delay_ms(10);
                    }
                    MotorDirection::Brake => {
                        let _ = in1_pin.set_high();
                        let _ = in2_pin.set_high();
                        FreeRtos::delay_ms(10);
                    }
                    MotorDirection::Forward => {
                        let _ = in2_pin.set_low();

                        if on_us > 0 {
                            let _ = in1_pin.set_high();
                            Ets::delay_us(on_us);
                        }

                        if off_us > 0 {
                            let _ = in1_pin.set_low();
                            Ets::delay_us(off_us);
                        }

                        FreeRtos::delay_ms(1);
                    }
                    MotorDirection::Reverse => {
                        let _ = in1_pin.set_low();

                        if on_us > 0 {
                            let _ = in2_pin.set_high();
                            Ets::delay_us(on_us);
                        }

                        if off_us > 0 {
                            let _ = in2_pin.set_low();
                            Ets::delay_us(off_us);
                        }

                        FreeRtos::delay_ms(1);
                    }
                }
            })?;

        info!("DRV8833 motor service started");
        Ok(Self { state })
    }

    pub fn set(&self, speed: u8, direction: MotorDirection) -> Result<(), String> {
        let clamped_speed = speed.min(100);

        self.state
            .lock()
            .map_err(|e| format!("Failed to lock motor state: {}", e))
            .map(|mut s| {
                s.speed = clamped_speed;
                s.direction = direction;
            })
    }

    pub fn get_speed(&self) -> u8 {
        self.state.lock().map(|s| s.speed).unwrap_or(0)
    }

    pub fn get_direction(&self) -> MotorDirection {
        self.state
            .lock()
            .map(|s| s.direction)
            .unwrap_or(MotorDirection::Coast)
    }
}
