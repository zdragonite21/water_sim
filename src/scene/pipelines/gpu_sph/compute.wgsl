@group(0) @binding(0) var<storage, read_write> data: array<u32>;

fn swap_inversions(a: u32, b: u32) {
    let num_items = arrayLength(&data);

    if a < num_items && b < num_items && data[a] > data[b] {
        let temp = data[a];
        data[a] = data[b];
        data[b] = temp;
    }
}

@compute
@workgroup_size(64)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    let pair_index = gid.x;

    // odd
    var a = pair_index * 2u + 1;
    var b = a + 1u;
    swap_inversions(a, b);

    storageBarrier();

    // even
    a = pair_index * 2u;
    b = a + 1u;
    swap_inversions(a, b);
}
