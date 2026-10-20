// An empty copy with virtual bases still runs its copy constructor in
// startup, rather than becoming a vptr-only static image; expect exit 0
struct V { virtual int value() { return 1; } };
struct A : virtual V {};
A first;
A second(first);
struct D : A {};
D original;
D copied(original);
int main() { return second.value() + copied.value() == 2 ? 0 : 1; }
