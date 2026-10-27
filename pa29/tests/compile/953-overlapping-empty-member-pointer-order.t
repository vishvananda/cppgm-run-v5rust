struct Empty {};
struct Small {
public:
    int first;
private:
    [[no_unique_address]] Empty empty;
public:
    int last;
    constexpr Small() : first(1), empty{}, last(2) {}
};
struct Large {
public:
    int first;
private:
    [[no_unique_address]] Empty empty;
public:
    int last, a, b, c, d, e, f, g;
    constexpr Large() : first(1), empty{}, last(2), a(0), b(0), c(0), d(0), e(0), f(0), g(0) {}
};
constexpr Small small{};
constexpr Large large{};
static_assert(&small.first < &small.last, "small byte owner");
static_assert(&large.first < &large.last, "large byte owner");
int main() { return 0; }
