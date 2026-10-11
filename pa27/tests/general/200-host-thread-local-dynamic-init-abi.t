// host-object link: N3485 3.7.2/3.6.2 + Itanium ABI thread_local wrapper/init
// (_ZTW/_ZTH): an extern thread_local with a dynamic initializer defined in
// another TU is initialized before its first odr-use from this TU, across
// cppgm++/host objects
