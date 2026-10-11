// N3485 7.1.2p1/p5, 9.2p8-p9, 9.5p2, 10.4p2: virtual only on member functions
// in their class (not a data member, friend or union member); pure-specifier
// only `= 0` on the in-class declaration; each virt-specifier once;
// must be rejected
struct A { virtual int x; };
int main() { return 0; }
