// N3485 7.2p5: an enumerator value of an enumeration with a fixed underlying
// type is a converted constant expression; 70000 narrows to short; must be
// rejected
enum class F : short { A = 70000 };
int main() { return (int)F::A; }
