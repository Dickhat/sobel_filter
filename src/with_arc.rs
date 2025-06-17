use std::{env, process, time::Instant, thread, sync::{Arc, Mutex}};
use image::{ImageBuffer, ImageReader, Rgb, Luma};


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

fn sobel_process(img: & Arc<ImageBuffer<Rgb<u8>, Vec<u8>>>, out_img: & Arc<Mutex<ImageBuffer<Luma<u8>, Vec<u8>>>>, number: u32, num_threads: u32)
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
    // Цикл обработки строк каждым потоком (приращение на число потоков)
    while row < img.height() - 1
    {
        for col in 1..img.width() - 1
        {
            gx = 0;
            gy = 0;

            for fil_x in 0..3
            {
                for fil_y in 0..3
                {
                    gx += img.get_pixel(col + fil_x - 1,row + fil_y - 1).0[0] as i32 * kernel_x[fil_x as usize][fil_y as usize];
                    gy += img.get_pixel(col + fil_x - 1,row + fil_y - 1).0[0] as i32 * kernel_y[fil_x as usize][fil_y as usize];
                }
            }
        
            // Конкуретный доступ к результативному изображению
            {
                let mut result_img = out_img.lock().unwrap();
                result_img.put_pixel(col,row, Luma([((gx * gx + gy * gy) as f32).sqrt().min(255.0) as u8]));
            }
        }
        row += num_threads;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let config = Configuration::new(&args).unwrap_or_else(|err|{
        println!("Problem parsing arguments: {err}");
        process::exit(1);
    }); 
    
    let image: Arc<ImageBuffer<Rgb<u8>, Vec<u8>>> = Arc::new(ImageReader::open(config.file_path)?.decode()?.to_rgb8());    
    let out_img: Arc<Mutex<ImageBuffer<Luma<u8>, Vec<u8>>>> = Arc::new(Mutex::new(image::ImageBuffer::new(image.width(), image.height())));

    let mut handles = vec![];
    
    let start = Instant::now();

    for number in 0..config.num_threads
    {
        let result_img = Arc::clone(&out_img);
        let image = Arc::clone(&image);
        let handle = thread::spawn(move || {
            sobel_process(&image, & result_img, number, config.num_threads);
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("Время выполнения c {:?} потоками: {:?}", config.num_threads, start.elapsed());

    out_img.lock().unwrap().save(config.out_path).unwrap_or_else(|err| {
        println!("Saving error: {err}");
        process::exit(1);
    });

    Ok(())
}
