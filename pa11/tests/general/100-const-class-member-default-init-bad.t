// N3485 8.5p6, 12.6.2p8: a constructor that default-initializes a const member
// of class type without a user-provided default constructor is ill-formed;
// must be rejected
struct P { int x; };
struct D { D(int) {} const P p; };
int main() { D d(1); return 0; }
