const PI: f32 = 3.14159265359;

fn smoothing_kernel(radius: f32, dst: f32) -> f32 {
    if dst >= radius {
        return 0.0;
    }

    var volume = PI * pow(radius, 4.0) / 6.0;
    var off = radius - dst;
    return off * off / volume;
}

fn smoothing_kernel_deriv(radius: f32, dst: f32) -> f32 {
    if dst >= radius {
        return 0.0;
    }
    var scale = 12.0 / (PI * pow(radius, 4.0));
    return (dst - radius) * scale;
}

// fn viscosity_smoothing_kernel(radius: f32, dst: f32) -> f32 {
//     if dst >= radius {
//         return 0.0;
//     }
//     let volume = PI * radius.powf(8.0) / 4.0;
//     let value = (radius * radius - dst * dst).max(0.0);
//     value * value * value / volume
// }
