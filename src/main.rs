use serialport::{SerialPort, SerialPortInfo, SerialPortType};
use simple_windows::{SimpleWindowApp, WindowHandle, Rect, Menu};


fn main() {
	let result = simple_windows::run_window_process("main", 320, 240, "Camera Serial Monitor", false, App::new());
	
	match result {
		Ok(_) => {},
		Err(err) => println!("{err}")
	}
	
}






pub struct App {
	pub available_ports: Vec<SerialPortInfo>,
	pub port: Option<Box<dyn SerialPort>>,
	pub signal_x: u32,
	pub signal_y: u32,
	pub leftover_byte: Option<u8>,
}

impl App {
	pub fn new() -> Self { Self {
		available_ports: vec![],
		port: None,
		signal_x: 0,
		signal_y: 0,
		leftover_byte: None,
	}}
	
	pub fn build_menu(&mut self, handle: &WindowHandle) -> Result<(), String> {
		self.available_ports = serialport::available_ports().unwrap_or_else(|err| panic!("Could not get info on serial ports: {err}"));
		
		let port_menu = Menu::new()?;
		
		for i in 1..=self.available_ports.len() {
			port_menu.add_item(i as u16, &format!("{} - {}", self.available_ports[i-1].port_name, &match &self.available_ports[i-1].port_type {
				SerialPortType::UsbPort(info) => match info.product.as_ref() {
					Some(s) => s,
					None => match info.manufacturer.as_ref() {
						Some(s) => s,
						None => "USB",
					}
				}
				SerialPortType::BluetoothPort => "Bluetooth",
				SerialPortType::PciPort => "PCI",
				SerialPortType::Unknown => "Unknown",
			}))?;
		}
		
		if let Some(port) = self.port.as_ref() {
			if let Some(name) = port.name() {
				for i in 1..=self.available_ports.len() {
					if self.available_ports[i-1].port_name == name {
						port_menu.set_item_check(i as u16, true)?;
						break
					}
				}
			}
		}
		
		if self.available_ports.len() == 0 {
			port_menu.add_item(0, "No serial connections found")?;
			port_menu.set_item_enable(0, false)?;
		}
		
		let menu = Menu::new()?;
		menu.add_submenu(port_menu, "Select Port")?;
		menu.add_item(0, "Rescan Ports")?;
		handle.set_menu(menu)?;
		
		handle.redraw_menu()?;
		return Ok(())
	}
	
	pub fn try_build_menu(&mut self, handle: &WindowHandle) {
		if let Err(err) = self.build_menu(handle) {
			println!("Error creating menu: {err}");
		}
	}
}

impl SimpleWindowApp for App {
	fn on_init(&mut self, handle: &WindowHandle) {
		self.try_build_menu(handle);
		handle.set_timer(1, 10);
	}
	
	fn on_command(&mut self, handle: &WindowHandle, _pixel_buffer: &mut [u8], _client_rect: &Rect, command_id: u16) {
		match command_id {
			0 => self.try_build_menu(handle),
			n => {
				if let Some(port) = self.port.as_ref() {
					if let Some(name) = port.name() {
						if self.available_ports[n as usize - 1].port_name == name {
							println!("Already listening to port {}", &name);
							return;
						}
					}
				}
				
				let port_name = &self.available_ports[n as usize - 1].port_name;
				let port_attempt = serialport::new(port_name, 3000000)
					.stop_bits(serialport::StopBits::One)
					.data_bits(serialport::DataBits::Eight)
					.open();
				
				match port_attempt {
					Ok(port) => {
						self.port = Some(port);
						let port_menu = handle.get_menu().unwrap().get_submenu(0).unwrap();
						for i in 1..=port_menu.item_count() {
							port_menu.set_item_check(i as u16, false).unwrap();
						}
						port_menu.set_item_check(n, true).unwrap();
						handle.request_redraw();
					}
					Err(e) => {
						println!("Could not open port {port_name}: {e}");
						self.try_build_menu(handle);
					}
				}
			}
		}
	}
	
	// fn on_paint(&mut self, handle: &WindowHandle, pixel_buffer: &mut [u8], client_rect: &Rect) {
	// 	// dbg!("draw");
		
	// 	// let mut i = 0;
	// 	// for y in 0..height {
	// 	// 	for x in 0..width {
	// 	// 		pixel_buffer[i|0] = (x as i32 * 255 / width) as u8;
	// 	// 		pixel_buffer[i|1] = (y as i32 * 255 / height) as u8;
	// 	// 		pixel_buffer[i|2] = 255 - (x as i32 * 255 / width) as u8;
				
	// 	// 		i += 4;
	// 	// 	}
	// 	// }
	// }
	
	fn on_timer(&mut self, handle: &WindowHandle, pixel_buffer: &mut [u8], client_rect: &Rect, timer_id: usize) {
		match timer_id {
			1 => {
				if let Some(port) = &mut self.port {
					match port.bytes_to_read() {
						Ok(n) => {
							let n = n & (!1);
							if n > 0 {
								let mut buf = vec![0; n as usize];
								match port.read(&mut buf) {
									Ok(_n) => {
										
										for data in buf.chunks_exact(2) {
											match data[0] {
												0 => {
													let width = client_rect.width();
													let height = client_rect.height();
													if self.signal_x >= width as u32 || self.signal_y >= height as u32 { continue }
													
													let i = ((self.signal_y * width as u32 + self.signal_x) as usize) << 2;
													
													pixel_buffer[i | ((self.signal_x & 1) + (self.signal_y & 1)) as usize] = data[1];
													
													self.signal_x += 1;
												}
												1 => {
													self.signal_x = 0;
													self.signal_y += 1;
												}
												2 => {
													self.signal_x = 0;
													self.signal_y = 0;
												}
												_ => {
													port.read(&mut vec![0u8; 1]).unwrap_or(0);
												}
											}
										}
										
										handle.request_redraw();
									}
									Err(e) => println!("Serial read error: {e}")
								}
							}
						}
						Err(e) => println!("Serial read error: {e}")
					}
				}
				
				handle.set_timer(1, 10);
				
			},
			_ => ()
		}
	}
}


