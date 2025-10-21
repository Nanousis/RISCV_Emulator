use std::{fs};

mod bus;
mod peripherals;
mod cpu;
mod constants;
mod types;

use peripherals::{Ram, UartNs16550a, TextMode};
use peripherals::ScreenHandle;
use bus::Bus;
use cpu::Cpu;
use constants::*;
use types::{Ctrl, CtrlMessage, ScreenMsg, ScreenType};
use crate::{bus::Device, peripherals::{ScreenCsr}};
use std::time::Instant;
use std::io;
use clap::{ArgAction, Parser};
// thread stuff
use std::sync::mpsc;
use std::thread;
use eframe::egui::{self};
use std::fs::File;
use std::io::{BufWriter, Write};

mod gui_app;
use gui_app::GUIApp;

#[derive(Parser, Debug)]
#[command(name = "RISC-V Emulator", version, about = "A simple RISC-V emulator in Rust", long_about = None)]
struct Args {
    program: String,
    /// Verbose (-v, -vv, -vvv)
    #[arg(short, long, action = ArgAction::Count)]
    verbose: u8,

    #[arg(short, long, default_value_t = 0)]
    limit : u64,
    /// Log instructions to a file
    /// Log instructions to a file
    #[arg(long, default_value = None)]
    log: Option<String>,
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



fn cpu_thread(cpu: &mut Cpu, args: &Args, textmode_frame: ScreenHandle, rx: &mpsc::Receiver<CtrlMessage>, screen_tx: mpsc::Sender<ScreenMsg>) {
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
    let logging_enabled = args.log.is_some();
    let mut writer = None;
    if logging_enabled {
        let file = File::create(args.log.as_ref().unwrap()).expect("Unable to create log file");
        writer = Some(BufWriter::with_capacity(64 * 1024, file));
        write!(writer.as_mut().unwrap(), "Emulation Trace  ").expect("Failed to write header");
    }
    let _batch = if verbose { 1 } else { 1000 };
    for _ in 0..(limit/_batch) {
        let mut input = String::new();
        if args.limit == 0 {
            io::stdin()
                .read_line(&mut input) // reads until Enter is pressed
                .expect("Failed to read line");
            if input.trim() == "q" || input.trim() == "b" {
                break;
            }
        }
        let event_log = cpu.tick(verbose, _batch, logging_enabled);
        if logging_enabled{

            for event in event_log {
                event.serialize(writer.as_mut().unwrap()).expect("Failed to write event");
            }
        }
        if let Ok(msg) = rx.try_recv() {
            match msg.command {
                Ctrl::RequestFrame => {
                    let frame_buff_enabled = cpu.read_mem(4, SCREEN_CSR_ENABLE) & 1 == 1;
                    let frame_buff_addr = cpu.read_mem(4, SCREEN_CSR_ADDR + 4);
                    // println!("Received frame request enabled:{} addr:0x{:08X}", frame_buff_enabled, frame_buff_addr);
                    if !frame_buff_enabled{
                        //fucking kill me.
                        if let Ok(buf) = textmode_frame.read() {
                            screen_tx.send(ScreenMsg { screen_type: ScreenType::TextMode, data: buf.clone() }).ok();
                        }
                    }
                    else{
                        // println!("Frame Buffer Addr: 0x{:08X}", frame_buff_addr);
                        let frame_size = SCREEN_WIDTH * SCREEN_HEIGHT * 2;
                        let mut frame_buff = vec![255; frame_size];
                        for i in 0..(frame_size/4) {
                            let data = cpu.read_mem(4, frame_buff_addr + (i*4) as u32);
                            let bytes = data.to_le_bytes();
                            frame_buff[i*4    ] = bytes[0];
                            frame_buff[i*4 + 1] = bytes[1];
                            frame_buff[i*4 + 2] = bytes[2];
                            frame_buff[i*4 + 3] = bytes[3];
                        }
                        let _ = screen_tx.send(ScreenMsg { screen_type: ScreenType::FrameBuffer, data: frame_buff } );
                    }
                }
                Ctrl::Stop => break,
            }
        }
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
    if let Ok(msg) = rx.recv() {
        match msg.command {
            Ctrl::RequestFrame => {
                let frame_buff_enabled = cpu.read_mem(4, SCREEN_CSR_ENABLE) & 1 == 1;
                let frame_buff_addr = cpu.read_mem(4, SCREEN_CSR_ADDR + 4);
                // println!("Received frame request enabled:{} addr:0x{:08X}", frame_buff_enabled, frame_buff_addr);
                if !frame_buff_enabled{
                    //fucking kill me.
                    if let Ok(buf) = textmode_frame.read() {
                        screen_tx.send(ScreenMsg { screen_type: ScreenType::TextMode, data: buf.clone() }).ok();
                    }
                }
                else{
                    // println!("Frame Buffer Addr: 0x{:08X}", frame_buff_addr);
                    let frame_size = SCREEN_WIDTH * SCREEN_HEIGHT * 2;
                    let mut frame_buff = vec![255; frame_size];
                    for i in 0..(frame_size/4) {
                        let data = cpu.read_mem(4, frame_buff_addr + (i*4) as u32);
                        let bytes = data.to_le_bytes();
                        frame_buff[i*4    ] = bytes[0];
                        frame_buff[i*4 + 1] = bytes[1];
                        frame_buff[i*4 + 2] = bytes[2];
                        frame_buff[i*4 + 3] = bytes[3];
                    }
                    let _ = screen_tx.send(ScreenMsg { screen_type: ScreenType::FrameBuffer, data: frame_buff } );
                }
            }
            Ctrl::Stop => println!("CPU thread stopping."),
        }
    }
}

fn main() -> eframe::Result {
    let args = Args::parse();
    println!("Loading program from: {:?}", args);

    let file_path = &args.program;
    let ram_init = parse_hex_file(file_path).expect("Failed to parse hex file");
    
    // Initiate the thread communication channels
    let (ctrl_tx, ctrl_rx) = mpsc::channel::<CtrlMessage>();
    let (screen_tx, screen_rx) = mpsc::channel::<ScreenMsg>();
    let (uart_tx, uart_rx) = mpsc::channel::<char>();
    
    // Cpu and bus initialization.
    let mut bus = Bus::new();

    let screen_csr = ScreenCsr::new();

    // SOMEHOW give it the screen csr struct
    let mut ram = Ram::new(1024 * 4096); // 4MB RAM
    for (i, &value) in ram_init.iter().enumerate() {
        ram.write(4, (i * 4) as u32, value).expect("Failed to write to RAM");
    }
    let vga_text_mode = TextMode::new();
    let textmode_frame = vga_text_mode.handle();
    bus.add_region(SCREEN_CSR_ADDR, 8, Box::new(screen_csr));
    bus.add_region(UART0_BASE, 0x0000_000F, Box::new(UartNs16550a::new(uart_tx)));
    bus.add_region(RAM_BASE, ram.size(), Box::new(ram));
    bus.add_region(VGA_TEXT_MODE_BASE, 1216*2, Box::new(vga_text_mode));
    let mut cpu = Cpu::new(bus, 0x8000_0000);
    

    let thread_handle = thread::spawn(move || {
        cpu_thread(&mut cpu, &args, textmode_frame, &ctrl_rx, screen_tx);
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
            app.screen_rx = Some(screen_rx);
            app.ctrl_tx = Some(ctrl_tx.clone());
            app.uart_rx = Some(uart_rx);
            Ok(Box::new(app))
        }),
    )?;
    // Not that when using verbose you have to press enter to quit (to exit the other thread).
    thread_handle.join().unwrap();
    Ok(())
}
