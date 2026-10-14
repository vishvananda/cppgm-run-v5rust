// N3485 7.1.6.4: compound placeholders, cv removal and forwarding collapse.
int x;
int* p = &x;
auto*& r = p;
const auto* const& c = p;
auto&& forward = x;
const int array[3] = {1, 2, 3};
const auto& whole = array;
auto decayed = array;
auto scalar = 1, *pointer = p;
auto identity(int value) -> decltype(value) { return value; }
auto (*function_pointer)(int) -> int = identity;
