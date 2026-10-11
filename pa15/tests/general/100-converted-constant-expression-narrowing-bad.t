// N3485 5.19p3, 14.3.2p5, 6.4.2p2: a converted constant expression may not
// narrow: template<unsigned char C> with 256 is ill-formed; must be rejected
template<unsigned char C> struct X {};
X<256> x;
int main() { return 0; }
