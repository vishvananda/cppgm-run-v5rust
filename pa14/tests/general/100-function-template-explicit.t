template<typename T>
T id(T x) { return x; }

int main() { return id<int>(5) - 5; }
