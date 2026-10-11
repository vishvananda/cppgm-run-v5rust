// host-object link: N3485 5.19/3 + 3.7.2: the address of a thread_local object
// is not an address constant, so p and q are dynamically initialized with the
// (main) thread's &v, not a static relocation to the TLS image
