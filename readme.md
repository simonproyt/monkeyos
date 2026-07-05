MonkeyOS - An experimental webos that uses the wasm runtime and rust to work

## Whats working
- basic filesystem that is backed by indexdb and supports the caching of wasm binaries for faster loading cause downloading 20 megs of wasm is kinda slow
- basic process management
- a basic shell with rust uutils backed commands and unix pipes support
- a cli text editor
## Whats not working/added
- any 3d graphics 
- sound and video playback
- displaying the actual user name instead of somebody in ls -l but this is an uutils problem because it does that if you compile to non unix targets so there is no easy way to fix this
- the ui not breaking after a small change
- any icons and stuff cause we havent added that to our rendering engine
- unit tests or any automated testing to auto check for bugs

## How to compile:
1. First you need to install the nodejs presquits by running npm i
2. After that you need to install the stable rust via rustup and add the wasm32-wasip1 toolchain
3. You need to run npm dev and that will compile the kernel and the js stuff and start a webserver and you can try it in a webgpu compatible browser
## Try it out (you will need a browser that support webgpu or enable it in your browsers flags)

https://simonproyt.github.io/monkeyos/

## Many thanks for the developers of the rust uutils project becuase they saved me a lot of time