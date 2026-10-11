template<typename T>
int f() { return 1; }

template<>
int f<int>() { return 2; }

int main() { return f<int>(); }
