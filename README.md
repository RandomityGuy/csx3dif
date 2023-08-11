# csx3dif
A complete rewrite of csx2dif in Rust, fixing the critical issues.  
Converts Torque Constructor CSX files to Torque DIF interior format.  

## Usage
```
Usage: csx3dif.exe [OPTIONS] <FILEPATH>

Arguments:
  <FILEPATH>

Options:
  -s, --silent
          Silent, don't print output
  -d, --dif-version <DIF_VERSION>
          Dif version to export to [default: 0]
  -e, --engine-version <ENGINE_VERSION>
          Engine version to export to [default: mbg] [possible values: mbg, tge, tgea, t3d]
      --mb <MB>
          Make DIF optimized for Marble Blast [default: true] [possible values: true, false]
      --bsp <BSP>
          BSP algorithm to use [default: exhaustive] [possible values: sampling, exhaustive, none]
      --epsilon-point <EPSILON_POINT>
          Epsilon for points to be considered the same [default: 0.000001]
      --epsilon-plane <EPSILON_PLANE>
          Epsilon for planes to be considered the same [default: 0.00001]
  -h, --help
          Print help
  -V, --version
          Print version
```


## Features
- Entity support
- PathedInterior support (as subobjects), no path_node support yet.
- Automatic splitting of large CSX files into multiple DIF files
- (mostly) Working balaned BSP tree for raycasts.
- Target any version of the Torque Game Engine/Torque3D

## Not supported features
- Lighting and Lightmaps
- Portals
- Static Meshes
- Triggers
- Path Nodes
- Vehicle Collision


## FAQ
### Conversion times are too long
This is because creating the BSP tree for raycasts takes a while, you can skip building the BSP tree with `--bsp none`. You will not be able to raycast, but conversion will be much faster. If your map does not need any raycasts, this option is advisible to reduce the filesize.

### Raycasts are not working for some faces
This is because of the epsilon value of planes being too small, increase them with `--epsilon-plane <EPSILON_PLANE>`. However, this can cause some faces to be incorrectly merged with other faces, causing collision bugs. So you need to find a balance between working collision and working raycasts.

### In marble blast, where do raycasts happen?
Marble Blast Gold/Platinum does not make use of raycasts. PlatinumQuest makes use of raycasts for drawing the cannon trajectory as well as the "Drop to Ground" option in the editor.

## Build

### Terminal
```
cargo build --release
```

### Web
```
cd csx3dif-web
wasm-pack build --target web
cd js/csx3dif-web
npm run dev
```

### Credits
Thanks to HiGuy for the libdif library to parse/write dif files.