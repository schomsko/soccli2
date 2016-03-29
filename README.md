# soccli2

A cli for soundcloud listening. Currently OSX only.
This is a pet project to start learning Rust.

## Dependencies
    - OSX 
    - VLC
    - soundcloud api key in a file at path: '~/.soccli2'

## What i have learned so far is:
    ### 1. Input parsing
        It is not so easy to find out about the numericity of a string within a match block. 
    ### 2. Compiling the hyper crate on OSX El Capitan can be tricky. 
        Best you install openssl via brew and gain environmental variables with something like:
        export OPENSSL_LIB_DIR=/usr/local/opt/openssl/lib 
        export DEP_OPENSSL_INCLUDE=/usr/local/opt/openssl/include
    ### 3. Rust
        Seems nice. 
