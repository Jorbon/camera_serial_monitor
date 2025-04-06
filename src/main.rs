use std::{fs::File, io::{Cursor, Write}, time::{Duration, Instant}};

use serialport::{SerialPort, SerialPortInfo};
use speedy2d::{color::Color, dimen::{UVec2, Vec2}, font::{Font, TextLayout, TextOptions}, image::{ImageFileFormat, ImageHandle, ImageSmoothingMode}, shape::Rectangle, window::{MouseButton, WindowHelper}};



struct MyWindowHandler {
	pub available_ports: Vec<SerialPortInfo>,
	pub port: Option<Box<dyn SerialPort>>,
	pub image: Option<ImageHandle>,
	pub jpeg_buffer: Vec<u8>,
	pub sidebar_width: f32,
	pub sidebar_item_height: f32,
	pub font: Font,
	pub last_scan_time: Instant,
	pub mouse_position: Vec2,
	pub ff_byte: bool,
}

impl MyWindowHandler {
	fn rescan_ports(&mut self) {
		self.available_ports = serialport::available_ports().unwrap_or_else(|err| panic!("Could not get info on serial ports: {err}"));
		self.last_scan_time = Instant::now();
	}
	
	
}


impl speedy2d::window::WindowHandler for MyWindowHandler {
	fn on_start(&mut self, _helper: &mut WindowHelper<()>, info: speedy2d::window::WindowStartupInfo) {
		self.rescan_ports();
		self.sidebar_width = info.viewport_size_pixels().x as f32 * 0.25;
	}
	
	fn on_resize(&mut self, _helper: &mut WindowHelper<()>, _size_pixels: speedy2d::dimen::UVec2) {
		
	}
	
	fn on_mouse_move(&mut self, _helper: &mut WindowHelper<()>, position: Vec2) {
		self.mouse_position = position;
	}
	
	fn on_mouse_button_down(&mut self, _helper: &mut WindowHelper<()>, button: MouseButton) {
		match button {
			MouseButton::Left => {
				if self.mouse_position.x >= 0.0 && self.mouse_position.x < self.sidebar_width {
					let item_index = (self.mouse_position.y / self.sidebar_item_height) as usize;
					match item_index {
						0 => (),
						1 => {
							self.port = None;
						}
						n => if let Some(info) = self.available_ports.get(n - 2) {
							
							if let Some(port) = self.port.as_ref() {
								if let Some(name) = port.name() {
									if info.port_name == name {
										println!("Already listening to port {}", &name);
										return;
									}
								}
							}
							
							let port_attempt = serialport::new(&info.port_name, 115200)
								.stop_bits(serialport::StopBits::One)
								.data_bits(serialport::DataBits::Eight)
								.open();
							
							match port_attempt {
								Ok(port) => {
									self.port = Some(port);
								}
								Err(e) => {
									println!("Could not open port {}: {e}", info.port_name);
								}
							}
						}
						
						
					}
				}
			}
			_ => ()
		}
	}
	
	fn on_draw(&mut self, helper: &mut WindowHelper<()>, graphics: &mut speedy2d::Graphics2D) {
		
		if Instant::now().duration_since(self.last_scan_time).as_secs_f32() >= 1.0 {
			self.rescan_ports();
		}
		
		
		if let Some(port) = self.port.as_mut() {
			match port.bytes_to_read() {
				Err(e) => println!("Serial bytes_to_read error: {e}"),
				Ok(0) => (),
				Ok(n) => {
					let mut buf = vec![0; n as usize];
					
					match port.read(&mut buf) {
						Err(e) => println!("Serial bytes_to_read error: {e}"),
						Ok(0) => (),
						Ok(n) => {
							
							for byte in &buf[0..n] {
								if self.ff_byte {
									match byte {
										0xd8 => {
											self.jpeg_buffer.clear();
											self.jpeg_buffer.push(0xff);
											
										}
										0xd9 => {
											self.jpeg_buffer.push(0xd9);
											let cursor = Cursor::new(self.jpeg_buffer.clone());
											
											match graphics.create_image_from_file_bytes(Some(ImageFileFormat::JPEG), ImageSmoothingMode::Linear, cursor) {
												Ok(image) => self.image = Some(image),
												Err(_e) => {
													// println!("Jpeg decoding error");
												}
											}
											
											self.jpeg_buffer.clear();
										}
										_ => ()
									}
								}
								
								if !self.jpeg_buffer.is_empty() {
									self.jpeg_buffer.push(*byte);
								}
								
								self.ff_byte = *byte == 0xff;
							}
							// println!("{:?}", &self.jpeg_buffer[previous_end..]);
						}
					}
				}
			}
		}
		
		
		// if self.image.is_none() {
		// 	self.image = Some(graphics.create_image_from_file_bytes(Some(ImageFileFormat::JPEG), ImageSmoothingMode::Linear, cursor).unwrap());
		// }
		
		let size = helper.get_size_pixels();
		let width = size.x as f32;
		let height = size.y as f32;
		
		graphics.clear_screen(Color::from_gray(0.1));
		
		graphics.draw_rectangle(Rectangle::from_tuples((0.0, 0.0), (self.sidebar_width, height)), Color::from_gray(0.3));
		
		
		let mut y = 0.0;
		let left_gap = self.sidebar_item_height * 0.25;
		let font_size = self.sidebar_item_height * 0.8;
		let text_lower = self.sidebar_item_height * 0.07;
		
		graphics.draw_rectangle(Rectangle::from_tuples((0.0, y), (self.sidebar_width, y + self.sidebar_item_height)), Color::from_gray(0.2));
		graphics.draw_text((left_gap, y + text_lower), Color::from_gray(0.9), &self.font.layout_text("Select Port", font_size, TextOptions::new()));
		y += self.sidebar_item_height;
		
		if self.port.is_none() {
			graphics.draw_rectangle(Rectangle::from_tuples((0.0, y), (self.sidebar_width, y + self.sidebar_item_height)), Color::from_rgb(0.2, 0.3, 0.5));
		}
		graphics.draw_text((left_gap, y + text_lower), Color::from_gray(0.9), &self.font.layout_text("None", font_size, TextOptions::new()));
		y += self.sidebar_item_height;
		
		for info in &self.available_ports {
			if let Some(port) = self.port.as_ref() {
				if let Some(name) = port.name() {
					if info.port_name == name {
						graphics.draw_rectangle(Rectangle::from_tuples((0.0, y), (self.sidebar_width, y + self.sidebar_item_height)), Color::from_rgb(0.2, 0.3, 0.5));
					}
				}
			}
			graphics.draw_text((left_gap, y + text_lower), Color::from_gray(0.9), &self.font.layout_text(&info.port_name, font_size, TextOptions::new()));
		}
		
		
		
		
		if let Some(image) = &self.image {
			let size = image.size();
			let area_width = width - self.sidebar_width;
		
			if size.x as f32 / size.y as f32 >= area_width / height {
				let margin = (height - size.y as f32 / size.x as f32 * area_width) * 0.5;
				graphics.draw_rectangle_image(Rectangle::from_tuples((self.sidebar_width, margin), (width, height - margin)), &image);
			} else {
				let margin = (area_width - size.x as f32 / size.y as f32 * height) * 0.5;
				graphics.draw_rectangle_image(Rectangle::from_tuples((self.sidebar_width + margin, 0.0), (width - margin, height)), &image);
			}
		}
		
		
		helper.request_redraw();
	}
}



fn main() {
	
	let window_size = UVec2::new(840, 480);
	let window = speedy2d::Window::new_centered("Camera Serial Monitor", window_size).unwrap();
	
	let window_handler = MyWindowHandler {
		available_ports: vec![],
		port: None,
		image: None,
		jpeg_buffer: vec![],
		sidebar_width: 200.0,
		sidebar_item_height: 50.0,
		font: Font::new(include_bytes!("OpenSans-Regular.ttf")).unwrap(),
		last_scan_time: Instant::now(),
		mouse_position: Vec2::new(0.0, 0.0),
		ff_byte: false,
	};
	
	window.run_loop(window_handler);
}








// Old raw/rgb rendering code

	// const HREF_PATTERN: &[u8] = &[1, 1, 254, 254];
	// const VSYNC_PATTERN: &[u8] = &[2, 2, 253, 253];
	// let width = client_rect.width() as u32;
	// let height = client_rect.height() as u32;

	// for byte in buf {
		
	// 	// RGB 565
	// 	if self.signal_x < width && self.signal_y < height {
	// 		let i = ((self.signal_y * width as u32 + self.signal_x) as usize) << 2;
			
	// 		if self.first_byte_in_pair {
	// 			pixel_buffer[i | 0] = byte << 3;
	// 			pixel_buffer[i | 1] = (byte & 0b11100000) >> 3;
				
	// 			self.first_byte_in_pair = false;
	// 		} else {
	// 			pixel_buffer[i | 1] |= byte << 5;
	// 			pixel_buffer[i | 2] = byte & 0b11111000;
				
	// 			self.first_byte_in_pair = true;
	// 			self.signal_x += 1;
	// 		}
	// 	}
		
	// 	// Raw
	// 	// if self.signal_x < width && self.signal_y < height {
	// 	// 	let color_offset = ((self.signal_x & 1) + (self.signal_y & 1)) as usize;
	// 	// 	let i = ((self.signal_y * width as u32 + self.signal_x) as usize) << 2;
	// 	// 	pixel_buffer[i | color_offset] = byte;
	// 	// 	self.signal_x += 1;
	// 	// }
		
		
	// 	if byte == VSYNC_PATTERN[self.vsync_match] {
	// 		self.href_match = 0;
	// 		self.vsync_match += 1;
	// 		if self.vsync_match == VSYNC_PATTERN.len() {
	// 			self.signal_x = 0;
	// 			self.signal_y = 0;
	// 			self.vsync_match = 0;
	// 			self.first_byte_in_pair = true;
	// 		}
	// 		continue
	// 	}
		
	// 	self.vsync_match = 0;
		
	// 	if byte == HREF_PATTERN[self.href_match] {
	// 		self.href_match += 1;
	// 		if self.href_match == HREF_PATTERN.len() {
	// 			self.signal_x = 0;
	// 			self.signal_y += 1;
	// 			self.href_match = 0;
	// 			self.first_byte_in_pair = true;
	// 		}
	// 		continue
	// 	}
		
	// 	self.href_match = 0;
	// }


