use std::{fmt::Display, io::{Cursor, Write}, time::Instant};

use serialport::{SerialPort, SerialPortInfo};
use speedy2d::{color::Color, dimen::{UVec2, Vec2}, font::{Font, TextLayout, TextOptions}, image::{ImageDataType, ImageHandle, ImageSmoothingMode}, shape::Rectangle, window::{MouseButton, WindowHelper}};


const N_CAMERAS: usize = 3;

const JPEG_START: [u8; 10] = [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46];

const CLOCK_DIVISOR_MAX: u8 = 32;
const RESOLUTION_MAX: u8 = 6;



const BACKGROUND_COLOR: Color = Color::from_gray(0.1);
const BAR_COLOR: Color = Color::from_gray(0.2);
const LABEL_COLOR: Color = Color::from_gray(0.2);
const BUTTON_COLOR: Color = Color::from_gray(0.4);
const SLIDER_COLOR: Color = Color::from_gray(0.3);
const HIGHLIGHT_COLOR: Color = Color::from_rgb(0.3, 0.4, 0.6);
const INTERACT_COLOR: Color = Color::from_rgb(0.4, 0.5, 0.8);
const TEXT_COLOR: Color = Color::from_gray(0.9);





pub fn hexprint(buf: &[u8]) {
	for byte in buf {
		if *byte >= 33 && *byte <= 126 {
			print!("{} ", *byte as char);
		} else {
			print!("{byte:02x} ");
		}
	}
	println!();
}



#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Resolution {
	S160x120   = 0,
	S320x240   = 1,
	S640x480   = 2,
	S800x600   = 3,
	S1024x768  = 4,
	S1600x1200 = 5,
	T352x288   = 6,
	T1280x1024 = 7,
}

impl Resolution {
	pub fn from_u8(n: u8) -> Option<Self> {
		match n {
			0 => Some(Self::S160x120),
			1 => Some(Self::S320x240),
			2 => Some(Self::S640x480),
			3 => Some(Self::S800x600),
			4 => Some(Self::S1024x768),
			5 => Some(Self::S1600x1200),
			_ => None
		}
	}
}

impl Display for Resolution {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(match self {
			Self::S160x120   => "160 x 120",
			Self::S320x240   => "320 x 240",
			Self::S640x480   => "640 x 480",
			Self::S800x600   => "800 x 600",
			Self::S1024x768  => "1024 x 768",
			Self::S1600x1200 => "1600 x 1200",
			Self::T352x288   => "352 x 288",
			Self::T1280x1024 => "1280 x 1024",
		})
	}
}




struct MyWindowHandler {
	pub font: Font,
	pub sidebar_width: f32,
	pub sidebar_item_height: f32,
	pub mouse_position: Vec2,
	pub interacting_with: Option<Setting>,
	
	pub available_ports: Vec<SerialPortInfo>,
	pub port: Option<Box<dyn SerialPort>>,
	pub last_scan_time: Instant,
	
	pub images: [Option<ImageHandle>; N_CAMERAS],
	pub jpeg_buffer: Vec<u8>,
	pub jpeg_start_progress: usize,
	pub next_image_index: Option<usize>,
	
	pub clock_divisor: u8,
	pub resolution: Resolution,
}

#[derive(PartialEq, Eq)]
pub enum Setting {
	SelectPort(Option<String>),
	ClockDivisor,
	Resolution,
	SendSettings,
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
		
		if let Some(setting) = &self.interacting_with {
			let slider_width = self.sidebar_width * 0.1;
			match setting {
				Setting::ClockDivisor => {
					self.clock_divisor = (((position.x - slider_width * 0.5) / (self.sidebar_width - slider_width)).clamp(0.0, 1.0) * (CLOCK_DIVISOR_MAX - 1) as f32) as u8 + 1;
				}
				Setting::Resolution => {
					if let Some(r) = Resolution::from_u8((((position.x - slider_width * 0.5) / (self.sidebar_width - slider_width)).clamp(0.0, 1.0) * (RESOLUTION_MAX - 1) as f32) as u8) {
						self.resolution = r;
					}
				}
				Setting::SendSettings => (),
				Setting::SelectPort(_) => (),
			}
		}
		
	}
	
	fn on_mouse_button_down(&mut self, helper: &mut WindowHelper<()>, button: MouseButton) {
		let size = helper.get_size_pixels();
		// let width = size.x as f32;
		let height = size.y as f32;
		
		match button {
			MouseButton::Left => {
				
				if self.mouse_position.x >= 0.0 && self.mouse_position.x < self.sidebar_width {
					let item_index_from_top = (self.mouse_position.y / self.sidebar_item_height) as usize;
					match item_index_from_top {
						0 => (),
						1 => {
							self.port = None;
							self.interacting_with = Some(Setting::SelectPort(None));
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
							
							self.interacting_with = Some(Setting::SelectPort(Some(info.port_name.clone())));
						}
						
					}
					
					
					let item_index_from_bottom = ((height - self.mouse_position.y) / self.sidebar_item_height) as usize;
					match item_index_from_bottom {
						2 => {
							let slider_width = self.sidebar_width * 0.1;
							let slider_position = (self.sidebar_width - slider_width) * self.resolution as u8 as f32 / (RESOLUTION_MAX - 1) as f32;
							
							if !(self.mouse_position.x >= slider_position && self.mouse_position.x < slider_position + slider_width) {
								if let Some(r) = Resolution::from_u8((((self.mouse_position.x - slider_width * 0.5) / (self.sidebar_width - slider_width)).clamp(0.0, 1.0) * (RESOLUTION_MAX - 1) as f32).round() as u8) {
									self.resolution = r;
								}
							}
							self.interacting_with = Some(Setting::Resolution);
						}
						1 => {
							let slider_width = self.sidebar_width * 0.1;
							let slider_position = (self.sidebar_width - slider_width) * (self.clock_divisor - 1) as f32 / (CLOCK_DIVISOR_MAX - 1) as f32;
							
							if !(self.mouse_position.x >= slider_position && self.mouse_position.x < slider_position + slider_width) {
								self.clock_divisor = (((self.mouse_position.x - slider_width * 0.5) / (self.sidebar_width - slider_width)).clamp(0.0, 1.0) * (CLOCK_DIVISOR_MAX - 1) as f32).round() as u8 + 1;
							}
							self.interacting_with = Some(Setting::ClockDivisor);
						}
						0 => {
							if let Some(port) = self.port.as_mut() {
								port.write(&[((0b111 & self.resolution as u8) << 5) | (0b11111 & ((self.clock_divisor - 1) >> 1))]).unwrap();
								// println!("Sent speed command");
							}
							self.interacting_with = Some(Setting::SendSettings);
						}
						_ => ()
					}
					
				}
			}
			_ => ()
		}
	}
	
	fn on_mouse_button_up(&mut self, _helper: &mut WindowHelper<()>, button: MouseButton) {
		match button {
			MouseButton::Left => {
				self.interacting_with = None;
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
							// println!("\nfirst data after :");
							// hexprint(&buf[0..n]);
							
							
							for byte in &buf[0..n] {
								
								if let Some(image_index) = self.next_image_index {
									
									if self.jpeg_start_progress < JPEG_START.len() {
										// dbg!(byte, self.jpeg_start_progress);
										if self.jpeg_start_progress == 0 {
											
											for i in 2..9 {
												if *byte == JPEG_START[i] {
													
													self.jpeg_start_progress = i + 1;
													
													// dbg!("1", self.jpeg_start_progress, self.jpeg_start_progress_is_ambiguous);
													break
												}
											}
											
										} else if *byte == self.jpeg_buffer[self.jpeg_start_progress] {
											self.jpeg_start_progress += 1;
											// dbg!("2", self.jpeg_start_progress);
										} else {
											self.jpeg_start_progress = 0;
											// dbg!("4");
										}
										
									} else {
										self.jpeg_buffer.push(*byte);
										let n = self.jpeg_buffer.len() - 2;
										
										if self.jpeg_buffer[n] == 0xff {
											if self.jpeg_buffer[n + 1] == 0xd8 {
												self.jpeg_buffer.truncate(JPEG_START.len());
												self.jpeg_start_progress = 2;
											} else if self.jpeg_buffer[n + 1] == 0xd9 {
												
												let cursor = Cursor::new(self.jpeg_buffer.clone());
												
												match image::ImageReader::with_format(cursor, image::ImageFormat::Jpeg).decode() {
													Ok(image) => {
														let img = graphics.create_image_from_raw_pixels(ImageDataType::RGB, ImageSmoothingMode::Linear, (image.width(), image.height()), image.as_bytes()).unwrap();
														self.images[image_index] = Some(img);
													}
													Err(e) => {
														println!("From camera {}: {e}", match self.next_image_index {
															Some(n) => n.to_string(),
															None => "unknown".to_string()
														});
														
														// println!("\n\n{e}\n");
														// hexprint(&self.jpeg_buffer);
													}
												}
												
												// println!("{:?}", self.jpeg_buffer.len());
												self.jpeg_buffer.truncate(JPEG_START.len());
												self.jpeg_start_progress = 0;
												self.next_image_index = None;
											}
										}
									}
									
								} else {
									if let Some(image_index) = byte.checked_sub(b'0') {
										if (image_index as usize) < N_CAMERAS {
											self.next_image_index = Some(image_index as usize);
										}
									}
								}
								
							}
							
							
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
		
		graphics.clear_screen(BACKGROUND_COLOR);
		
		graphics.draw_rectangle(Rectangle::from_tuples((0.0, 0.0), (self.sidebar_width, height)), BAR_COLOR);
		
		
		let mut y = 0.0;
		let left_gap = self.sidebar_item_height * 0.25;
		let font_size = self.sidebar_item_height * 0.8;
		let text_lower = self.sidebar_item_height * 0.07;
		
		graphics.draw_rectangle(Rectangle::from_tuples((0.0, y), (self.sidebar_width, y + self.sidebar_item_height)), LABEL_COLOR);
		graphics.draw_text((left_gap, y + text_lower), TEXT_COLOR, &self.font.layout_text("Select Port", font_size, TextOptions::new()));
		y += self.sidebar_item_height;
		
		if self.port.is_none() {
			graphics.draw_rectangle(Rectangle::from_tuples((0.0, y), (self.sidebar_width, y + self.sidebar_item_height)), 
				if self.interacting_with == Some(Setting::SelectPort(None)) {INTERACT_COLOR} else {HIGHLIGHT_COLOR}
			);
		}
		graphics.draw_text((left_gap, y + text_lower), TEXT_COLOR, &self.font.layout_text("None", font_size, TextOptions::new()));
		y += self.sidebar_item_height;
		
		for info in &self.available_ports {
			if let Some(port) = self.port.as_ref() {
				if let Some(name) = port.name() {
					if info.port_name == name {
						graphics.draw_rectangle(Rectangle::from_tuples((0.0, y), (self.sidebar_width, y + self.sidebar_item_height)), 
						if self.interacting_with == Some(Setting::SelectPort(Some(name))) {INTERACT_COLOR} else {HIGHLIGHT_COLOR}
					);
					}
				}
			}
			graphics.draw_text((left_gap, y + text_lower), TEXT_COLOR, &self.font.layout_text(&info.port_name, font_size, TextOptions::new()));
		}
		
		y = height - 3.0 * self.sidebar_item_height;
		
		
		let slider_width = self.sidebar_width * 0.1;
		
		graphics.draw_rectangle(Rectangle::from_tuples((0.0, y), (self.sidebar_width, y + self.sidebar_item_height)), SLIDER_COLOR);
		
		let slider_position = (self.sidebar_width - slider_width) * self.resolution as u8 as f32 / (RESOLUTION_MAX - 1) as f32;
		graphics.draw_rectangle(Rectangle::from_tuples((slider_position, y), (slider_position + slider_width, y + self.sidebar_item_height)), 
			if let Some(Setting::Resolution) = self.interacting_with {INTERACT_COLOR} else {HIGHLIGHT_COLOR}
		);
		
		let text = self.font.layout_text(&self.resolution.to_string(), font_size, TextOptions::new());
		graphics.draw_text(((self.sidebar_width - text.width()) * 0.5, y + text_lower), TEXT_COLOR, &text);
		y += self.sidebar_item_height;
		
		
		graphics.draw_rectangle(Rectangle::from_tuples((0.0, y), (self.sidebar_width, y + self.sidebar_item_height)), SLIDER_COLOR);
		
		let slider_position = (self.sidebar_width - slider_width) * (self.clock_divisor - 1) as f32 / (CLOCK_DIVISOR_MAX - 1) as f32;
		graphics.draw_rectangle(Rectangle::from_tuples((slider_position, y), (slider_position + slider_width, y + self.sidebar_item_height)), 
			if let Some(Setting::ClockDivisor) = self.interacting_with {INTERACT_COLOR} else {HIGHLIGHT_COLOR}
		);
		
		let text = self.font.layout_text(&format!("Divisor: {}", self.clock_divisor), font_size, TextOptions::new());
		graphics.draw_text(((self.sidebar_width - text.width()) * 0.5, y + text_lower), TEXT_COLOR, &text);
		y += self.sidebar_item_height;
		
		
		graphics.draw_rectangle(Rectangle::from_tuples((0.0, y), (self.sidebar_width, y + self.sidebar_item_height)), 
			if let Some(Setting::SendSettings) = self.interacting_with {INTERACT_COLOR} else {BUTTON_COLOR}
		);
		let text = self.font.layout_text("Send Settings", font_size, TextOptions::new());
		graphics.draw_text(((self.sidebar_width - text.width()) * 0.5, y + text_lower), TEXT_COLOR, &text);
		// y += self.sidebar_item_height;
		
		
		
		for i in 0..self.images.len() {
			if let Some(Some(image)) = &self.images.get(i) {
				let size = image.size();
				let area_width = (width - self.sidebar_width) / 3.0;
				
				// if size.x as f32 / size.y as f32 >= area_width / height {
					let margin = (height - size.y as f32 / size.x as f32 * area_width) * 0.5;
					graphics.draw_rectangle_image(Rectangle::from_tuples((self.sidebar_width + area_width * i as f32, margin), (self.sidebar_width + area_width * (i + 1) as f32, height - margin)), &image);
				// } else {
				// 	let margin = (area_width - size.x as f32 / size.y as f32 * height) * 0.5;
				// 	graphics.draw_rectangle_image(Rectangle::from_tuples((self.sidebar_width + margin, 0.0), (width - margin, height)), &image);
				// }
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
		images: [const {None}; 3],
		jpeg_buffer: JPEG_START.to_vec(),
		jpeg_start_progress: 0,
		next_image_index: None,
		sidebar_width: 200.0,
		sidebar_item_height: 50.0,
		font: Font::new(include_bytes!("OpenSans-Regular.ttf")).unwrap(),
		last_scan_time: Instant::now(),
		mouse_position: Vec2::new(0.0, 0.0),
		clock_divisor: CLOCK_DIVISOR_MAX,
		resolution: Resolution::S160x120,
		interacting_with: None,
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


