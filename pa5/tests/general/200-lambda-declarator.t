int main() { auto f = [](int x) mutable noexcept(x) -> int { return x; }; }
