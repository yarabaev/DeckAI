// Renders a single VRT case to stdout as SVG.
// Usage: render <case-name> [slide-number]
//
// Used by cross_process.sh for cross-process determinism checks.
// Exits with code 1 on any error; never panics.
use slideglance_vrt::{render_case, CASES};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: render <case-name> [slide-number]");
        std::process::exit(1);
    }
    let case_name = &args[1];
    let slide_filter: Option<u32> = args
        .get(2)
        .and_then(|s| s.parse().ok());

    let case = match CASES.iter().find(|c| c.name == *case_name) {
        Some(c) => c,
        None => {
            eprintln!("unknown case: {case_name}");
            eprintln!(
                "available: {}",
                CASES.iter().map(|c| c.name).collect::<Vec<_>>().join(", ")
            );
            std::process::exit(1);
        }
    };

    let slides = match render_case(case) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("render error for {case_name}: {e}");
            std::process::exit(1);
        }
    };

    for (n, svg) in &slides {
        if slide_filter.map_or(true, |f| f == *n) {
            print!("{svg}");
        }
    }
}
