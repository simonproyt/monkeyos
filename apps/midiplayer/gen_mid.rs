use midly::{Smf, Format, Header, Timing, Track, TrackEvent, TrackEventKind, MidiMessage, MetaMessage};
use std::fs::File;

fn main() {
    let mut smf = Smf::new(Header::new(Format::SingleTrack, Timing::Metrical(480.into())));
    let mut track = Track::new();
    
    // Program Change (Acoustic Grand Piano)
    track.push(TrackEvent { delta: 0.into(), kind: TrackEventKind::Midi { channel: 0.into(), message: MidiMessage::ProgramChange { program: 0.into() } } });
    
    let notes = [60, 62, 64, 65, 67, 69, 71, 72];
    for note in notes {
        track.push(TrackEvent { delta: 0.into(), kind: TrackEventKind::Midi { channel: 0.into(), message: MidiMessage::NoteOn { key: note.into(), vel: 64.into() } } });
        track.push(TrackEvent { delta: 480.into(), kind: TrackEventKind::Midi { channel: 0.into(), message: MidiMessage::NoteOff { key: note.into(), vel: 64.into() } } });
    }
    
    track.push(TrackEvent { delta: 0.into(), kind: TrackEventKind::Meta(MetaMessage::EndOfTrack) });
    smf.tracks.push(track);
    
    let mut buf = Vec::new();
    smf.write(&mut buf).unwrap();
    
    // convert to data URI
    use std::io::Write;
    let b64 = base64::encode(buf);
    println!("data:audio/midi;base64,{}", b64);
}
