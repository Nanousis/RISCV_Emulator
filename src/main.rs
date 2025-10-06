use std::fs;

mod bus;
mod peripherals;
mod cpu;

use peripherals::{Ram, UartNs16550a, TextMode};
use bus::Bus;
use cpu::Cpu;
use crate::bus::Device;
use std::time::Instant;
use std::io;
use clap::{builder::Str, ArgAction, Parser};
// thread stuff
use std::sync::mpsc;
use std::thread;
use eframe::egui::{self, TextureHandle};

mod gui_app;
use gui_app::GUIApp;

// Register names mapping
const REGISTER_NAMES: [&str; 32] = [
    "zero", "ra",  "sp",  "gp",  "tp",  "t0", "t1", "t2",
    "s0",   "s1",  "a0",  "a1",  "a2",  "a3", "a4", "a5",
    "a6",   "a7",  "s2",  "s3",  "s4",  "s5", "s6", "s7",
    "s8",   "s9",  "s10", "s11", "t3",  "t4", "t5", "t6"
];
#[derive(Parser, Debug)]
#[command(name = "RISC-V Emulator", version, about = "A simple RISC-V emulator in Rust", long_about = None)]
struct Args {
    program: String,
    /// Verbose (-v, -vv, -vvv)
    #[arg(short, long, action = ArgAction::Count)]
    verbose: u8,

    #[arg(short, long, default_value_t = 0)]
    limit : u64,
}
fn parse_hex_file(file_path: &str) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(file_path)?;
    let tokens = contents.split_whitespace();
    let nums: Vec<u32> = tokens
        .map(|s| {
            let s = s.trim();
            let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
            let s_no_underscores: String = s.chars().filter(|&c| c != '_').collect();
            u32::from_str_radix(&s_no_underscores, 16).unwrap()
        })
        .collect();
    Ok(nums)
}

struct CtrlMessage {
    command: Ctrl,
    data: String,
}
enum Ctrl {
    Data,
    Stop,
}

fn cpu_thread(cpu: &mut Cpu, args: &Args, rx: &mpsc::Receiver<CtrlMessage>) {
    // std::thread::sleep(std::time::Duration::from_secs(2));
    let start = Instant::now();
    let verbose = args.verbose > 0;
    // Start the CPU
    let limit = if args.limit == 0 { u64::MAX } else { args.limit };
    if args.limit == 0 {
        println!("Running in interactive mode. Press Enter to step, 'q/b' to quit.");
    } else {
        println!("Running for {} cycles.", limit);
    }
    println!("------------\n");
    for _ in 0..limit {
        let mut input = String::new();
        if args.limit == 0 {
            io::stdin()
                .read_line(&mut input) // reads until Enter is pressed
                .expect("Failed to read line");
            if input.trim() == "q" || input.trim() == "b" {
                break;
            }
        }
        if let Ok(msg) = rx.try_recv() {
            match msg.command {
                Ctrl::Data => {
                    unimplemented!();
                }
                Ctrl::Stop => break,
            }
        }
        cpu.tick(verbose);
    }
    if verbose {
        for (i, &name) in REGISTER_NAMES.iter().enumerate() {
            let reg_data = cpu.read_reg(i);
            println!("x{} ({:>3}): 0x{:08X}({})", i, name, reg_data, reg_data);
        }
    }
    let duration = start.elapsed();
    println!("\n------------");
    println!("CPU execution time: {:?}", duration);
}

fn main() -> eframe::Result {
    let args = Args::parse();
    print!("Loading program from: {:?}\n", args);

    let file_path = &args.program;
    let ram_init = parse_hex_file(file_path).expect("Failed to parse hex file");
    
    // Initiate the thread communication channels
    let (ctrl_tx, ctrl_rx) = mpsc::channel::<CtrlMessage>();
    let (mem_tx, mem_rx) = mpsc::channel::<CtrlMessage>();
    
    // Cpu and bus initialization.
    let mut bus = Bus::new();
    let mut ram = Ram::new(1024 * 4096); // 4MB RAM
    for (i, &value) in ram_init.iter().enumerate() {
        ram.write(4, (i * 4) as u32, value).expect("Failed to write to RAM");
    }
    let mut vga_text_mode = TextMode::new(mem_tx);

    bus.add_region(0x1000_0000, 0x0000_000F, Box::new(UartNs16550a));
    bus.add_region(0x0000_0000, ram.size(), Box::new(ram));
    bus.add_region(0x8800_0000, 1216*2, Box::new(vga_text_mode));
    let mut cpu = Cpu::new(bus, 0x0000_0000);
    

    let thread_handle = thread::spawn(move || {
        cpu_thread(&mut cpu, &args,&ctrl_rx);
    });
    
    // can use try receive to not block
    // let received = rx.recv().unwrap();
    // println!("Got: {received}");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 800.0]).with_min_inner_size([800.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "RISCV Emulator",
        options,
        Box::new(move |_cc| {
            let mut app = GUIApp::default();
            app.mem_rx = Some(mem_rx);
            app.ctrl_tx = Some(ctrl_tx.clone());
            Ok(Box::new(app))
        }),
    )?;
    // Not that when using verbose you have to press enter to quit (to exit the other thread).
    thread_handle.join().unwrap();
    Ok(())
}
