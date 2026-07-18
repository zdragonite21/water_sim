// this example creates an array with 10 key-value (u32,f32) pairs and sorts them on the gpu
use std::num::NonZeroU32;

use wgpu_sort::{
    utils::{download_buffer, download_buffer2, guess_workgroup_size, upload_to_buffer},
    GPUSorter,
};

#[pollster::main]
async fn main() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

    let adapter = wgpu::util::initialize_adapter_from_env_or_default(&instance, None)
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .unwrap();
    let subgroup_size = guess_workgroup_size(&device, &queue)
        .await
        .expect("could not find a valid subgroup size");
    println!("using subgroup size {subgroup_size}");
    let sorter = GPUSorter::new(&device, subgroup_size);

    let n = 36;
    let sort_buffers = sorter.create_sort_buffers(&device, NonZeroU32::new(n).unwrap());

    let keys_scrambled: Vec<u32> = vec![
        65085, 130621, 196157, 261693, 327229, 392765, 458301, 523837, 589373, 65337, 130873,
        196409, 261945, 327481, 393017, 458553, 524089, 589625, 130944, 196480, 327552, 393088,
        65442, 130978, 196514, 262050, 327586, 393122, 458658, 524194, 589730, 589824, 589824,
        589824, 589824, 589824,
    ];

    let values_scrambled: Vec<u32> = keys_scrambled.iter().map(|v| 5).collect();

    loop {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        upload_to_buffer(
            &mut encoder,
            &sort_buffers.keys(),
            &device,
            keys_scrambled.as_slice(),
        );
        upload_to_buffer(
            &mut encoder,
            &sort_buffers.values(),
            &device,
            values_scrambled.as_slice(),
        );

        println!(
            "before: {:?}",
            keys_scrambled
                .iter()
                .zip(values_scrambled.iter())
                .collect::<Vec<(_, _)>>()
        );

        // sorter.sort(&mut encoder, &sort_buffers);
        sorter.sort(&mut encoder, &queue, &sort_buffers, None);

        // wait for sorter to finish
        let idx = queue.submit([encoder.finish()]);
        /*
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            })
            .unwrap();
            */

        // keys buffer has padding at the end
        // so we only download the "valid" data
        download_buffer2(&sort_buffers.keys(), &device, &queue);
        download_buffer2(&sort_buffers.values(), &device, &queue);
    }
}
