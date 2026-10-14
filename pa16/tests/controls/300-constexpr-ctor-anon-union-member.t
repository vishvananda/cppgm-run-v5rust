// N3485 7.1.5p4/9.5p5: a constexpr constructor that initializes a member of an
// anonymous union member (a variant member) initializes that union;
// expect exit 0
union K1 { union { unsigned m4; }; char m8; constexpr K1() : m4(7) {} };
constexpr K1 k1{};
int main() { return k1.m4 == 7 ? 0 : 1; }
