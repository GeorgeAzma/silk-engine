use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use midir::{Ignore, MidiInput};

use crate::prelude::ResultAny;

#[derive(Event, Debug, Clone)]
pub enum MidiEvent {
    NoteOff {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    PolyphonicAftertouch {
        channel: u8,
        note: u8,
        pressure: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    ChannelAftertouch {
        channel: u8,
        pressure: u8,
    },
    PitchBend {
        channel: u8,
        value: i16,
    },
    System {
        status: u8,
        data: Vec<u8>,
    },
    Raw {
        timestamp: u64,
        bytes: Vec<u8>,
    },
}

#[derive(Resource)]
pub struct Midi {
    connection: Arc<Mutex<midir::MidiInputConnection<()>>>,
    events: Arc<Mutex<VecDeque<MidiEvent>>>,
    port_name: String,
}

impl Midi {
    pub fn new() -> ResultAny<Self> {
        Self::with_port(0)
    }

    pub fn with_port(port_index: usize) -> ResultAny<Self> {
        let mut midi_in = MidiInput::new("silk-engine-midi")?;
        midi_in.ignore(Ignore::None);

        let in_ports = midi_in.ports();
        let port = in_ports.get(port_index).ok_or_else(|| {
            format!(
                "no MIDI input port at index {port_index}; available ports: {}",
                in_ports.len()
            )
        })?;

        let port_name = midi_in.port_name(port)?;
        let events = Arc::new(Mutex::new(VecDeque::new()));
        let events_clone = events.clone();

        let connection = midi_in.connect(
            port,
            "silk-engine-midi-read",
            move |timestamp, message, _| {
                if let Some(event) = parse_message(timestamp, message) {
                    events_clone.lock().unwrap().push_back(event);
                }
            },
            (),
        )?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            events,
            port_name,
        })
    }

    pub fn ports() -> ResultAny<Vec<String>> {
        let mut midi_in = MidiInput::new("silk-engine-midi")?;
        midi_in.ignore(Ignore::None);

        let mut ports = Vec::new();
        for port in midi_in.ports() {
            ports.push(midi_in.port_name(&port)?);
        }
        Ok(ports)
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    pub fn drain(&self) -> Vec<MidiEvent> {
        self.events.lock().unwrap().drain(..).collect()
    }
}

pub struct MidiPlugin;

impl Default for MidiPlugin {
    fn default() -> Self {
        Self
    }
}

impl Plugin for MidiPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(midi) = Midi::new() {
            app.insert_resource(midi);
        }

        app.add_systems(Update, pump_midi);
    }
}

fn parse_message(timestamp: u64, message: &[u8]) -> Option<MidiEvent> {
    let status = *message.first()?;
    let kind = (status & 0b1111_0000) >> 4;
    let channel = status & 0b0000_1111;
    let data = &message[1..];

    Some(match kind {
        0b1000 => {
            let [note, velocity]: [u8; 2] = data.get(0..2)?.try_into().ok()?;
            MidiEvent::NoteOff {
                channel,
                note,
                velocity,
            }
        }
        0b1001 => {
            let [note, velocity]: [u8; 2] = data.get(0..2)?.try_into().ok()?;
            if velocity == 0 {
                MidiEvent::NoteOff {
                    channel,
                    note,
                    velocity,
                }
            } else {
                MidiEvent::NoteOn {
                    channel,
                    note,
                    velocity,
                }
            }
        }
        0b1010 => {
            let [note, pressure]: [u8; 2] = data.get(0..2)?.try_into().ok()?;
            MidiEvent::PolyphonicAftertouch {
                channel,
                note,
                pressure,
            }
        }
        0b1011 => {
            let [controller, value]: [u8; 2] = data.get(0..2)?.try_into().ok()?;
            MidiEvent::ControlChange {
                channel,
                controller,
                value,
            }
        }
        0b1100 => {
            let program = *data.first()?;
            MidiEvent::ProgramChange { channel, program }
        }
        0b1101 => {
            let pressure = *data.first()?;
            MidiEvent::ChannelAftertouch { channel, pressure }
        }
        0b1110 => {
            let [lsb, msb]: [u8; 2] = data.get(0..2)?.try_into().ok()?;
            let value = ((msb as u16) << 7 | lsb as u16) as i16 - 8192;
            MidiEvent::PitchBend { channel, value }
        }
        0b1111 => MidiEvent::System {
            status,
            data: data.to_vec(),
        },
        _ => MidiEvent::Raw {
            timestamp,
            bytes: message.to_vec(),
        },
    })
}

fn pump_midi(world: &mut World) {
    let events = match world.get_resource::<Midi>() {
        Some(midi) => midi.drain(),
        None => return,
    };

    for event in events {
        world.trigger(event);
    }
}