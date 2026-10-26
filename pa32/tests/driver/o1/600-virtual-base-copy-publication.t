// Copy initialization retains runtime startup and demanded base-entry order.
struct V { virtual int value() { return 1; } };
struct A : virtual V {};
A first;
A second(first);
struct D : A {};
D original;
D copied(original);
int main() { return second.value() + copied.value() - 2; }
