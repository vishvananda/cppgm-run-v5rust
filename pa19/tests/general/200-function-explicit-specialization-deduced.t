template<typename T>
int f(T) { return 1; }

template<>
int f<int>(int) { return 2; }

int main() { return f(0); }
