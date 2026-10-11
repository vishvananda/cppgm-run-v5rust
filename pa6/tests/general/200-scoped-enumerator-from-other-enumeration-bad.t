// N3485 7.2p5, 7.2p10: a scoped enumerator does not convert implicitly, so it
// cannot initialize another enumeration's enumerator; must be rejected
enum class A : unsigned char { X = 1 };
enum class B : unsigned char { X = A::X };
int main() { return (int)B::X; }
