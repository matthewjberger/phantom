# Rendering

It's finally time to build one of the most interesting parts of the game engine... the renderer!

We will be using the [wgpu](https://github.com/gfx-rs/wgpu) graphics API. It's a safe, portable GPU abstraction in Rust that implements the [WebGPU API](https://www.w3.org/TR/webgpu/). It supports a variety of backends and allows us to render our game on multiple operating systems using their preferred API's (Metal on MacOS, vulkan on Linux, DirectX12 on Windows, etc). This will prevent us from having to write multiple backends!

