// host-object link: GNU weak attribute, N3485 3.6.2p2, 3.7.2: when a strong
// definition of a dynamically initialized thread_local variable in one unit
// replaces a weak one in another, the program has one object, initialized once
// by the strong definition's initializer, whichever unit uses it first
