// Function parameter names in result expressions compare by position.
template<class T> auto identity(T value) -> decltype(value);
template<class U> auto identity(U renamed) -> decltype(renamed) { return renamed; }
long selected(double) { return 7; }
template<class T> auto result(T value) -> decltype(selected(value)) { return selected(value); }
int selected(int) { return 3; }
template<class U> auto result(U renamed) -> decltype(selected(renamed));
int main() { return identity(9) == 9 && result(0) == 7 ? 0 : 1; }
