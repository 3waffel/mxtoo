# MXTOO

include it in flake.nix

```nix
mxtoo.url = "github:3waffel/mxtoo";
```

then include the following content in your config

```nix
imports = [
    mxtoo.nixosModules.mxtoo
];

services.mxtoo = {
    enable = true;
    port = 7999;
};
```
