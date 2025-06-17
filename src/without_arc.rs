use std::{env, process, time::Instant, thread};
use image::{ImageBuffer, ImageReader, Rgb, Luma};

#[derive(Copy, Clone)]
struct RawImageConstPtr(*const ImageBuffer<Rgb<u8>, Vec<u8>>);

#[derive(Copy, Clone)]
struct RawImageMutPtr(*mut ImageBuffer<Luma<u8>, Vec<u8>>);

unsafe impl Send for RawImageConstPtr {}
unsafe impl Sync for RawImageConstPtr {}

unsafe impl Send for RawImageMutPtr {}
unsafe impl Sync for RawImageMutPtr {}

struct Configuration {
    file_path: String,
    out_path: String,
    num_threads: u32,
}

impl Configuration {
    fn new(args: &[String]) -> Result<Configuration, &'static str>
    {
        if args.len() < 4 {
            return Err("Not enough arguments");
        }

        dbg!(args);

        Ok(Configuration{ 
                        file_path: args[1].clone(), 
                        out_path: args[2].clone(), 
                        num_threads: args[3].parse::<u32>().unwrap()
                        }
        )
    }
}

fn sobel_process(img: &RawImageConstPtr, out_img: &RawImageMutPtr, number: u32, num_threads: u32)
{
    let mut gx: i32;
    let mut gy: i32;

    // Ядро фильтра Собела по оси X
    let kernel_x: [[i32; 3]; 3] = [[-1, 0, 1],
                                  [-2, 0, 2],
                                  [-1, 0, 1]];
    
    // Ядро фильтра Собела по оси Y
    let kernel_y: [[i32; 3]; 3] = [[1, 2, 1],
                                  [0, 0, 0],
                                  [-1, -2, -1]];


    let mut row = number + 1;

    unsafe 
    {
        // Цикл обработки строк каждым потоком (приращение на число потоков)
        while row < (*img.0).height() - 1
        {
            for col in 1..(*img.0).width() - 1
            {
                gx = 0;
                gy = 0;

                for fil_x in 0..3
                {
                    for fil_y in 0..3
                    {
                        gx += (*img.0).get_pixel(col + fil_x - 1,row + fil_y - 1).0[0] as i32 * kernel_x[fil_x as usize][fil_y as usize];
                        gy += (*img.0).get_pixel(col + fil_x - 1,row + fil_y - 1).0[0] as i32 * kernel_y[fil_x as usize][fil_y as usize];
                    }
                }
            
                // Не конкуретный доступ к выходному изображению (Каждый поток отвечает за свои строки number)
                {
                    (*out_img.0).put_pixel(col,row, Luma([((gx * gx + gy * gy) as f32).sqrt().min(255.0) as u8]));
                }
            }
            row += num_threads;
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let config = Configuration::new(&args).unwrap_or_else(|err|{
        println!("Problem parsing arguments: {err}");
        process::exit(1);
    }); 
    
    let image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageReader::open(config.file_path)?.decode()?.to_rgb8();    
    let mut out_img: ImageBuffer<Luma<u8>, Vec<u8>> = image::ImageBuffer::new(image.width(), image.height());

    let rawp_image = RawImageConstPtr(&image as *const ImageBuffer<Rgb<u8>, Vec<u8>>);
    let rawp_out_img = RawImageMutPtr(&mut out_img as *mut ImageBuffer<Luma<u8>, Vec<u8>>);

    let mut handles = vec![];
    
    let start = Instant::now();

    for number in 0..config.num_threads
    {
        let handle = thread::spawn(move || {
            sobel_process(&rawp_image, &rawp_out_img, number, config.num_threads);
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("Время выполнения c {:?} потоками: {:?}", config.num_threads, start.elapsed());

    out_img.save(config.out_path).unwrap_or_else(|err| {
        println!("Saving error: {err}");
        process::exit(1);
    });

    Ok(())
}
