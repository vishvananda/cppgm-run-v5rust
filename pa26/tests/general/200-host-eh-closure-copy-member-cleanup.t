// N3485 5.1.2p19, 12.8p15, 15.2p2: a closure's implicitly-defined copy
// constructor copies its (unnamed) capture members in order; when a later
// member's copy throws, the members already copied are destroyed. A plain
// struct's implicit copy constructor alike
