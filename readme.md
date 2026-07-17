MonkeyOS - An experimental webos that uses the wasm runtime and rust to work

## Whats working
- basic filesystem that is backed by indexdb and supports the caching of wasm binaries for faster loading cause downloading 20 megs of wasm is kinda slow (back now but in a seprate db so it dosent slow down the normal fs)
- basic process management
- a basic shell with rust uutils backed commands and unix pipes support
- a cli text editor
- unicode emoji based icons using the font rendering engine
- networking with the the fetch command and it tries to request the page themselves but its probably going to fail because of crocs but after that tries to request it via a proxy api
## Whats not working/added
- any 3d graphics 
- sound and video playback (the sound part works but video is kinda hard so its not planned in the near future)
- displaying the actual user name instead of somebody in ls -l but this is an uutils problem because it does that if you compile to non unix targets so there is no easy way to fix this
- the ui not breaking after a small change
- unit tests or any automated testing to auto check for bugs
## Gui Apps:
- File browser
- Terminal
- Notepad
- An image viewer
- A chiptune midi player
- Snake
all of them are quite basic but they are good enough for demoing the ui lib and they will get improved
## How to compile:
1. First you need to install the nodejs presquits by running npm i
2. After that you need to install the stable rust via rustup and add the wasm32-wasip1 toolchain
3. You need to run npm dev and that will compile the kernel and the js stuff and start a webserver and you can try it in a webgpu compatible browser
## Try it out (you will need a browser that support webgpu or enable it in your browsers flags)

https://simonproyt.github.io/monkeyos/
## Screenshots
the os is changing so much right now that the screenshots will get outdated quickly so i dont want to add them
## Many thanks for the developers of the rust uutils project becuase they saved me a lot of time