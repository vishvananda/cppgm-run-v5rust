// host-object link: N3485 3.7.2p2 (initialized before first odr-use; if
// constructed, destroyed on thread exit) + 3.6.2p2 (static initialization of d
// in every thread): a thread that uses z destroys its constant-initialized d
// too, as g++ and clang do
