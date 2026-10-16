// expect reject
template<class T> int declared(T) noexcept(T::missing);
int main() { return noexcept(declared(1)); }
