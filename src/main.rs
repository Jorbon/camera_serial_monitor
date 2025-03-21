use serialport::{SerialPort, SerialPortInfo, SerialPortType};
use simple_windows::{SimpleWindowApp, WindowHandle, Rect, Menu};


fn main() {
	simple_windows::run_window_process("main", 320, 240, "Camera Serial Monitor", false, App::new()).unwrap();
}










pub struct App {
	pub available_ports: Vec<SerialPortInfo>,
	pub port: Option<Box<dyn SerialPort>>,
	pub signal_x: u32,
	pub signal_y: u32,
	pub href_match: usize,
	pub vsync_match: usize,
	pub first_byte_in_pair: bool,
}

impl App {
	pub fn new() -> Self { Self {
		available_ports: vec![],
		port: None,
		signal_x: 0,
		signal_y: 0,
		href_match: 0,
		vsync_match: 0,
		first_byte_in_pair: true,
	}}
	
	pub fn build_menu(&mut self, handle: &WindowHandle) -> Result<(), String> {
		self.available_ports = serialport::available_ports().unwrap_or_else(|err| panic!("Could not get info on serial ports: {err}"));
		
		let port_menu = Menu::new()?;
		
		for i in 0..self.available_ports.len() {
			port_menu.add_item(i as u16, &format!("{} - {}", self.available_ports[i].port_name, &match &self.available_ports[i].port_type {
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
				for i in 0..self.available_ports.len() {
					if self.available_ports[i].port_name == name {
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
		menu.add_item(128, "Rescan Ports")?;
		menu.add_item(129, "Disconnect")?;
		handle.set_menu(menu)?;
		
		handle.redraw_menu()?;
		return Ok(())
	}
	
	pub fn try_build_menu(&mut self, handle: &WindowHandle) {
		if let Err(err) = self.build_menu(handle) {
			println!("Error creating menu: {err}");
		}
	}
	
	pub fn render_data(&mut self, handle: &WindowHandle, pixel_buffer: &mut [u8], client_rect: &Rect) {
		let port = match &mut self.port {
			None => return,
			Some(port) => port
		};
		
		let n = match port.bytes_to_read() {
			Err(e) => {
				println!("Serial read error: {e}");
				return
			}
			Ok(0) => return,
			Ok(n) => n
		};
		
		let mut buf = vec![0; n as usize];
		
		match port.read(&mut buf) {
			Err(e) => {
				println!("Serial read error: {e}");
				return
			}
			Ok(_n) => ()
		}
		
		
		const HREF_PATTERN: &[u8] = &[1, 1, 254, 254];
		const VSYNC_PATTERN: &[u8] = &[2, 2, 253, 253];
		let width = client_rect.width() as u32;
		let height = client_rect.height() as u32;
		
		for byte in buf {
			// RGB 565
			if self.signal_x < width && self.signal_y < height {
				let i = ((self.signal_y * width as u32 + self.signal_x) as usize) << 2;
				
				if self.first_byte_in_pair {
					pixel_buffer[i | 0] = byte << 3;
					pixel_buffer[i | 1] = (byte & 0b11100000) >> 3;
					
					self.first_byte_in_pair = false;
				} else {
					pixel_buffer[i | 1] |= byte << 5;
					pixel_buffer[i | 2] = byte & 0b11111000;
					
					self.first_byte_in_pair = true;
					self.signal_x += 1;
				}
			}
			
			// Raw
			// if self.signal_x < width && self.signal_y < height {
			// 	let color_offset = ((self.signal_x & 1) + (self.signal_y & 1)) as usize;
			// 	let i = ((self.signal_y * width as u32 + self.signal_x) as usize) << 2;
			// 	pixel_buffer[i | color_offset] = byte;
			// 	self.signal_x += 1;
			// }
			
			
			if byte == VSYNC_PATTERN[self.vsync_match] {
				self.href_match = 0;
				self.vsync_match += 1;
				if self.vsync_match == VSYNC_PATTERN.len() {
					self.signal_x = 0;
					self.signal_y = 0;
					self.vsync_match = 0;
					self.first_byte_in_pair = true;
				}
				continue
			}
			
			self.vsync_match = 0;
			
			if byte == HREF_PATTERN[self.href_match] {
				self.href_match += 1;
				if self.href_match == HREF_PATTERN.len() {
					self.signal_x = 0;
					self.signal_y += 1;
					self.href_match = 0;
					self.first_byte_in_pair = true;
				}
				continue
			}
			
			self.href_match = 0;
			
		}
		
		handle.request_redraw();
	}
}

impl SimpleWindowApp for App {
	fn on_init(&mut self, handle: &WindowHandle) {
		self.try_build_menu(handle);
		handle.set_timer(1, 20);
	}
	
	fn on_command(&mut self, handle: &WindowHandle, _pixel_buffer: &mut [u8], _client_rect: &Rect, command_id: u16) {
		match command_id {
			128 => self.try_build_menu(handle),
			129 => {
				self.port = None;
				self.try_build_menu(handle);
			}
			n => {
				if let Some(port) = self.port.as_ref() {
					if let Some(name) = port.name() {
						if self.available_ports[n as usize].port_name == name {
							println!("Already listening to port {}", &name);
							return;
						}
					}
				}
				
				let port_name = &self.available_ports[n as usize].port_name;
				let port_attempt = serialport::new(port_name, 115200)
					.stop_bits(serialport::StopBits::One)
					.data_bits(serialport::DataBits::Eight)
					.open();
				
				match port_attempt {
					Ok(port) => {
						self.port = Some(port);
						let port_menu = handle.get_menu().unwrap().get_submenu(0).unwrap();
						for i in 0..port_menu.item_count() {
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
				handle.set_timer(1, 10);
				self.render_data(handle, pixel_buffer, client_rect);
			}
			_ => ()
		}
	}
}


