// Nondependent result types are part of the template signature.
long selected(double);
template<class T> auto result(T) -> decltype(selected(0));
int selected(int);
template<class U> auto result(U) -> decltype(selected(0));
int main() { return sizeof(result(0)); }
