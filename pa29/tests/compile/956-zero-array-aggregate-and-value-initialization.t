struct Small { int first; int empty[0]; int last; };
constexpr Small small = Small();
static_assert(small.first == 0, "whole object");
struct Large {
    int first;
    int empty[0];
    int last;
    int a, b, c, d, e, f;
};
constexpr Large large{};
static_assert(large.first == 0 && large.last == 0 && large.f == 0, "aggregate");
int main() { return large.last; }
