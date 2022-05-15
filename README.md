# caldeira
![Rust](https://github.com/JeanCASPAR/caldeira/workflows/Rust/badge.svg)

A rust render engine based on Vulkan. Not production ready yet :warning:
For the moment I have a ready compute pipeline, try cargo run with a different compute.comp shader with the same interface,
it will output an image in ./image.png. The display does not work at this time. 

# Examples
* A checkerboard :
![Checkerboard](./examples/damier.png)

* A fire generated with perlin noise :
![Perlin fire](./examples/perlin_fire.png)

* Truchet pattern with circles :
![Truchet pattern with circles](./examples/truchet_pattern_circles.png)

* Truchet pattern with maze :
![Truchet pattern with maze](./examples/truchet_pattern_maze.png)

# Thanks
I adapted Perlin's algorithm for the Perlin noise, and I transposed code from https://vulkan-tutorial.com/.
I use https://github.com/ash-rs/ash for binding with Vulkan.
