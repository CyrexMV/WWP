//! Erzeugt die eingebetteten Schriften (WinAnsi-Teilmenge von Liberation Sans):
//! `cargo run --example schriften -- /usr/share/fonts/truetype/liberation assets/fonts`
use wwp::engine::ttf::Font;

fn main() {
    let mut a = std::env::args().skip(1);
    let src = a.next().unwrap_or_else(|| "/usr/share/fonts/truetype/liberation".into());
    let dst = a.next().unwrap_or_else(|| "assets/fonts".into());
    for name in ["LiberationSans-Regular.ttf", "LiberationSans-Bold.ttf"] {
        let data = std::fs::read(format!("{src}/{name}")).expect("Schrift nicht gefunden");
        let font = Font::parse(&data).expect("Schrift nicht lesbar");
        let sub = font.subset_winansi().expect("Teilmenge fehlgeschlagen");
        Font::parse(&sub).expect("Teilmenge nicht lesbar");
        std::fs::write(format!("{dst}/{name}"), &sub).unwrap();
        println!("{name}: {} → {} Bytes", data.len(), sub.len());
    }
}
