use std::error::Error;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::{Duration, Instant},
};
use uuid::Uuid;

use node::{Delay, Input, FromValueType};

use node::{
    device::{Action, Device, DeviceGroup, Devices},
    encoder,
    encoder::Encoder,
    updaters::EncoderDevices,
    AnyInputPin, InputPin, LedcDriver, LedcTimerDriver, Node, Pcnt, PinDriver, TimerConfig,
};

include!("credentials.inc");

fn main() -> Result<(), Box<dyn Error>> {
    let peripherals = Node::setup();

    let mut fan_node = Node {
        ssid: SSID.to_string(),
        password: PASSWORD.to_string(),
    };

    let fans = Vec::from([
        Device::build(
            Uuid::from_u128(0x262c2a63ca3140ba9c7c76a993e3c983),
            "roof vent".to_string(),
        )?
        .device_group(Some(DeviceGroup::Fan))?
        .available_actions(Vec::from([
            Action::On,
            Action::Off,
            Action::Up(None),
            Action::Down(None),
            Action::Min,
            Action::Max,
            Action::Set(0),
            Action::Reverse,
        ]))?,
        Device::build(
            Uuid::from_u128(0x09902974f4a548fda1bdd7ef19dc5db3),
            "kitchen fan".to_string(),
        )?
        .device_group(Some(DeviceGroup::Fan))?
        .available_actions(Vec::from([
            Action::On,
            Action::Off,
            Action::Up(None),
            Action::Down(None),
            Action::Min,
            Action::Max,
            Action::Set(0),
            Action::Reverse,
        ]))?,
    ]);

    let fans = Devices {
        devices: Arc::new(Mutex::new(fans)),
    };

    let louver = Arc::new(Mutex::new(
        Device::build(
            Uuid::from_u128(0x4d6edca3a8af4f2c8f937e627eb9fbb7),
            "vent louver".to_string(),
        )?
        .available_actions(Vec::from([Action::On, Action::Off]))?,
    ));

    let mut reverse_pins = vec![
        PinDriver::input(AnyInputPin::from(peripherals.pins.gpio12)).unwrap(),
        PinDriver::input(AnyInputPin::from(peripherals.pins.gpio46)).unwrap(),
    ];

    let fans_clone = fans.clone();
    let louver_clone = louver.clone();
    thread::spawn(move || {
        let delay_ms = 10;
        let mut pin_a_1 = peripherals.pins.gpio14;
        let mut pin_b_1 = peripherals.pins.gpio13;
        let mut pin_a_2 = peripherals.pins.gpio16;
        let mut pin_b_2 = peripherals.pins.gpio17;

        let mut encoders = Vec::from([
            Encoder::new(peripherals.pcnt0, &mut pin_a_1, &mut pin_b_1)
                .expect("Somehow the Encoder creation failed"),
            Encoder::new(peripherals.pcnt1, &mut pin_a_2, &mut pin_b_2)
                .expect("Somehow the Encoder creation failed"),
        ]);

        let length = { fans_clone.devices.lock().unwrap().len() };
        let mut last_encoder_values = vec![0; length];
        let mut last_encoder_times = vec![Instant::now(); length];
        let mut last_click_times: Vec<Option<Instant>> = vec![None; length];
        let mut last_click_time = None;
        let delay = Delay::new(delay_ms);
        loop {
            {
                let mut fans_guard = fans_clone.devices.lock().unwrap();
                for (
                    ((((fan, encoder), reverse_pin), last_encoder_time), last_encoder_value),
                    mut last_click_time,
                ) in fans_guard
                    .iter_mut()
                    .zip(encoders.iter_mut())
                    .zip(reverse_pins.iter_mut())
                    .zip(last_encoder_times.iter_mut())
                    .zip(last_encoder_values.iter_mut())
                    .zip(last_click_times.iter_mut())
                {
                    update_reversable_device_from_pin_click(
                        fan,
                        last_click_time,
                        reverse_pin,
                        50,
                        1000,
                    );
                    update_device_from_encoder(
                        fan,
                        encoder,
                        last_encoder_time,
                        last_encoder_value,
                        delay_ms.into(),
                    );
                }
            }
            {
                let mut louver_guard = louver_clone.lock().unwrap();
                update_two_way_switch_from_pin_click(
                    &mut louver_guard,
                    &mut last_click_time,
                    &mut reverse_pins[0],
                    1001,
                    5000,
                );
            }
            delay.delay_ms(10);
        }
    });

    let mut fans_clone = fans.clone();
    let louver_clone = louver.clone();
    thread::spawn(move || {
        let freqs = node::get_frequencies(&fans_clone);
        let freq = { louver_clone.lock().unwrap().freq_Hz.Hz() };
        let delay_ms = 100;
        let delay = Delay::new(delay_ms);
        let mut drivers = Vec::from([
            // roof vent
            LedcDriver::new(
                peripherals.ledc.channel0,
                LedcTimerDriver::new(
                    peripherals.ledc.timer0,
                    &TimerConfig::new().frequency(freqs[0]),
                )
                .unwrap(),
                peripherals.pins.gpio41,
            )
            .unwrap(),
            LedcDriver::new(
                peripherals.ledc.channel1,
                LedcTimerDriver::new(
                    peripherals.ledc.timer1,
                    &TimerConfig::new().frequency(freqs[0]),
                )
                .unwrap(),
                peripherals.pins.gpio42,
            )
            .unwrap(),
            // louver motor
            LedcDriver::new(
                peripherals.ledc.channel2,
                LedcTimerDriver::new(
                    peripherals.ledc.timer2,
                    &TimerConfig::new().frequency(freqs[1]),
                )
                .unwrap(),
                peripherals.pins.gpio45,
            )
            .unwrap(),
            LedcDriver::new(
                peripherals.ledc.channel3,
                LedcTimerDriver::new(
                    peripherals.ledc.timer3,
                    &TimerConfig::new().frequency(freqs[1]),
                )
                .unwrap(),
                peripherals.pins.gpio48,
            )
            .unwrap(),
            // kitchen fan
            /*LedcDriver::new(
                peripherals.ledc.channel4,
                LedcTimerDriver::new(
                    peripherals.ledc.timer2, 
                    &TimerConfig::new().frequency(freq)
                ).unwrap(),
                peripherals.pins.gpio41,
            )
            .unwrap(),
            LedcDriver::new(
                peripherals.ledc.channel5,
                LedcTimerDriver::new(
                    peripherals.ledc.timer2, 
                    &TimerConfig::new().frequency(freq)
                ).unwrap(),
                peripherals.pins.gpio42,
            )
            .unwrap(),*/
        ]);
        let max_duty_cycles = node::get_max_duty_cycles(&drivers);
        let mut old_duty_cycle = 0;
        let mut old_reversed = false;
        loop {
            {
                /*for ((fan, driver), max_duty) in fans_clone
                    .devices
                    .lock()
                    .unwrap()
                    .iter_mut()
                    .zip(drivers[..2].iter_mut())
                    .zip(max_duty_cycles[..2].iter())
                {
                    if fan.needs_hardware_duty_cycle_update() {
                        let duty_cycle = fan.get_and_update_duty_cycle(max_duty);
                        let _ = driver.set_duty(duty_cycle);
                    }
                }*/
                let mut fan = fans_clone.devices.lock().unwrap();
                let mut fan = fan.get_mut(0).unwrap();
                if fan.needs_hardware_duty_cycle_update() {
                    if fan.reversed {
                        if old_reversed == fan.reversed {
                            let _ = drivers[0].set_duty(0);
                            if old_duty_cycle == 0 {
                                let _ = drivers[1].set_duty(*max_duty_cycles.get(1).unwrap());
                                sleep(Duration::new(1, 0));
                            }
                            let duty_cycle = fan.get_and_update_duty_cycle(max_duty_cycles.get(1).unwrap());
                            let _ = drivers[1].set_duty(duty_cycle);
                            old_duty_cycle = duty_cycle;
                        } else {
                            let _ = drivers[0].set_duty(0);
                            let _ = drivers[1].set_duty(0);
                            sleep(Duration::new(5, 0));
                            let _ = drivers[1].set_duty(*max_duty_cycles.get(1).unwrap());
                            sleep(Duration::new(1, 0));
                            let duty_cycle = fan.get_and_update_duty_cycle(max_duty_cycles.get(1).unwrap());
                            let _ = drivers[1].set_duty(duty_cycle);
                            old_reversed = fan.reversed;
                        }
                    } else {
                        if old_reversed == fan.reversed {
                            let _ = drivers[1].set_duty(0);
                            if old_duty_cycle == 0 {
                                let _ = drivers[0].set_duty(*max_duty_cycles.get(0).unwrap());
                                sleep(Duration::new(1, 0));
                            }
                            let duty_cycle = fan.get_and_update_duty_cycle(max_duty_cycles.get(0).unwrap());
                            let _ = drivers[0].set_duty(duty_cycle);
                            old_duty_cycle = duty_cycle;
                        } else {
                            let duty_cycle = fan.get_and_update_duty_cycle(max_duty_cycles.get(0).unwrap());
                            let _ = drivers[0].set_duty(0);
                            let _ = drivers[1].set_duty(0);
                            sleep(Duration::new(5, 0));
                            let _ = drivers[0].set_duty(*max_duty_cycles.get(0).unwrap());
                            sleep(Duration::new(1, 0));
                            let _ = drivers[0].set_duty(duty_cycle);
                            old_reversed = fan.reversed;
                        }
                    }
                }
            }
            {
                let mut louver_guard = louver_clone.lock().unwrap();
                if louver_guard.needs_hardware_duty_cycle_update() {
                    let duty_cycle = louver_guard.get_and_update_duty_cycle(&max_duty_cycles[2]);
                    let _ = drivers[2].set_duty(max_duty_cycles[2]);
                }
            }
            dbg!(drivers[0].get_duty());
            dbg!(drivers[1].get_duty());
            //delay.delay_ms(delay_ms);
            delay.delay_ms(1000);
        }
    });

    let fans_clone = fans.clone();
    let _ = fan_node.run(fans_clone, peripherals.modem);

    Ok(())
}

fn update_two_way_switch_from_pin_click(
    device: &mut Device,
    last_click_time: &mut Option<Instant>,
    reverse_pin: &mut PinDriver<'static, AnyInputPin, Input>,
    min_click_duration_ms: u32,
    max_click_duration_ms: u32,
) {
    if reverse_pin.is_low() {
        if last_click_time.is_some() {
            let current_time = Instant::now();
            if current_time.duration_since(last_click_time.unwrap())
                > Duration::from_millis(min_click_duration_ms.into())
                && current_time.duration_since(last_click_time.unwrap())
                    < Duration::from_millis(max_click_duration_ms.into())
            {
                device.target_next_duty_cycle();
            }
            *last_click_time = None;
        }
    } else {
        if last_click_time.is_none() {
            *last_click_time = Some(Instant::now());
        }
    }
}

/// this is a copy from updater.rs
fn update_device_from_encoder(
    device: &mut Device,
    encoder: &mut Encoder,
    last_encoder_time: &mut Instant,
    last_encoder_value: &mut i32,
    delay_ms: u64,
) {
    let encoder_value = encoder.get_value().unwrap();
    if encoder_value != *last_encoder_value {
        let current_time = Instant::now();
        let time_since_last_check = current_time.duration_since(*last_encoder_time);
        if time_since_last_check > Duration::from_millis(delay_ms) {
            {
                if encoder_value > *last_encoder_value {
                    let _ = device.take_action(Action::Up(None));
                } else {
                    let _ = device.take_action(Action::Down(None));
                }
            }
            dbg!("update_device_from_encoder");
            *last_encoder_time = Instant::now();
        }
        *last_encoder_value = encoder_value;
        dbg!("2update_device_from_encoder");
    }
}

fn update_reversable_device_from_pin_click(
    device: &mut Device,
    last_click_time: &mut Option<Instant>,
    reverse_pin: &mut PinDriver<'static, AnyInputPin, Input>,
    min_click_duration_ms: u32,
    max_click_duration_ms: u32,
) {
    if reverse_pin.is_low() {
        if last_click_time.is_some() {
            let current_time = Instant::now();
            if current_time.duration_since(last_click_time.unwrap())
                > Duration::from_millis(min_click_duration_ms.into())
                && current_time.duration_since(last_click_time.unwrap())
                    < Duration::from_millis(max_click_duration_ms.into())
            {
                let _ = device.take_action(Action::Reverse);
            }
            *last_click_time = None;
        }
    } else {
        if last_click_time.is_none() {
            *last_click_time = Some(Instant::now());
        }
    }
}
