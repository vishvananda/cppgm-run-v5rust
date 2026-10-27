struct Implicit { int number; };
struct Explicit {
    int number;
    Explicit &operator=(const Explicit &) = default;
};
struct First {
    Implicit member;
    constexpr First &operator=(const First &) = default;
};
struct Second {
    Explicit member;
    constexpr Second &operator=(const Second &) = default;
};
int main() { return 0; }
