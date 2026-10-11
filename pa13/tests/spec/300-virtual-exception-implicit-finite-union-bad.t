// VALIDATION: compile-fail
// N3485 focus: 15.4 [except.spec] paragraphs 5 and 14.
// The implicit destructor permits both the base's int and the member's double.
struct Base { virtual ~Base() throw(int); };
struct Member { ~Member() throw(double); };
struct Derived : Base { Member member; };
int main() { return 0; }
