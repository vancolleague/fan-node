use std::{
    sync::{Arc, Mutex},
    thread,
};
use uuid::Uuid;

use node::{
    device::{Action, Behavior, Device, DeviceGroup, Devices},
    encoder,
    encoder::Encoder,
    updaters::EncoderDevices,
    AnyInputPin, InputPin, LedcDriver, LedcTimerDriver, Node, Pcnt, PinDriver, TimerConfig,
};

include!("credentials.inc");

fn main() {
    let peripherals = Node::setup();

    let mut fan_node = Node {
        ssid: SSID.to_string(),
        password: PASSWORD.to_string(),
    };

    let devices = Vec::from([
        Device::new(
            Uuid::from_u128(0x262c2a63ca3140ba9c7c76a993e3c983),
            "roof vent".to_string(),
        )
        .device_group(Some(DeviceGroup::Fan))
        .available_actions(Vec::from([
            Action::On,
            Action::Off,
            Action::Up(None),
            Action::Down(None),
            Action::Min,
            Action::Max,
            Action::Set(0),
            Action::Reverse,
        ]))
        .behavior(Behavior::ReversableSlider),
        Device::new(
            Uuid::from_u128(0x4d6edca3a8af4f2c8f937e627eb9fbb7),
            "vent louver".to_string(),
        )
        .available_actions(Vec::from([Action::On, Action::Off]))
        .behavior(Behavior::TwoWaySwitch),
        Device::new(
            Uuid::from_u128(0x09902974f4a548fda1bdd7ef19dc5db3),
            "kitchen fan".to_string(),
        )
        .device_group(Some(DeviceGroup::Fan))
        .available_actions(Vec::from([
            Action::On,
            Action::Off,
            Action::Up(None),
            Action::Down(None),
            Action::Min,
            Action::Max,
            Action::Set(0),
            Action::Reverse,
        ]))
        .behavior(Behavior::ReversableSlider),
    ]);

    let devices = Devices {
        devices: Arc::new(Mutex::new(devices)),
    };

    let mut devices_clone = devices.clone();
    thread::spawn(move || {
        let mut pin_a_1 = peripherals.pins.gpio4;
        let mut pin_b_1 = peripherals.pins.gpio5;
        let mut pin_a_2 = peripherals.pins.gpio16;
        let mut pin_b_2 = peripherals.pins.gpio17;
        let encoders = Vec::from([
            Encoder::new(peripherals.pcnt0, &mut pin_a_1, &mut pin_b_1)
                .expect("Somehow the Encoder creation failed"),
            Encoder::new(peripherals.pcnt1, &mut pin_a_2, &mut pin_b_2)
                .expect("Somehow the Encoder creation failed"),
        ]);
        let reverse_pins = vec![
            PinDriver::input(AnyInputPin::from(peripherals.pins.gpio3)).unwrap(),
            PinDriver::input(AnyInputPin::from(peripherals.pins.gpio46)).unwrap(),
        ];
        devices_clone.take_actions_reversible_slider_encoder(encoders, reverse_pins, 100);
    });
}
