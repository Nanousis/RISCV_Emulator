## Basic Example
To run you can use cargo. 
```
cargo run --release .\hex_programs\uart_test.hex --limit 27000000  
```

### Arguments

`<hex_file>` the file containing the assembly. The original PC is set in the main.rs (0x8000_0000). Support for custom reset PC will be added later.

`-v` is used to show the instruction run

`--limit <instr_num>` to run <instr_num> instructions. If it is 0 then you can run instruction by instruction by pressing enter.
