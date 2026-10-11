// N3485 8.5.1p2, 15.2p2, 5.3.4p18: when a copy or move constructor run while
// aggregate-initializing a class prvalue (an argument, an allocated object, an
// array element, a member, a return value) throws, the members already
// initialized are destroyed and allocated storage is freed, so no object stays
// alive; each case runs with an exception at every construction in turn
